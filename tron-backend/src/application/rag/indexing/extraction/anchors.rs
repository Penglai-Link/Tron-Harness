//! Model-selected references resolve only to continuous, immutable source text.
//! Persisted graphs still contain literal quotes and pass the same validator.
use serde_json::{Value, json};

pub(super) struct Anchors<'a> {
    source: &'a str,
    ranges: Vec<std::ops::Range<usize>>,
}

impl<'a> Anchors<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        let mut ranges = Vec::new();
        let (mut start, mut count) = (0, 0);
        for (offset, c) in source.char_indices() {
            count += 1;
            let end = offset + c.len_utf8();
            let sentence = "\n。！？!?；;".contains(c)
                || (c == '.' && source[end..].chars().next().is_none_or(char::is_whitespace));
            if sentence || count >= 160 {
                ranges.push(start..end);
                start = end;
                count = 0;
            }
        }
        if start < source.len() {
            ranges.push(start..source.len());
        }
        Self { source, ranges }
    }

    pub(super) fn prompt(&self) -> String {
        Value::Array(
            self.ranges
                .iter()
                .enumerate()
                .map(|(i, r)| json!({"id":i+1,"text":&self.source[r.clone()]}))
                .collect(),
        )
        .to_string()
    }

    fn resolve(&self, value: &Value) -> Result<&'a str, String> {
        let object = value
            .as_object()
            .filter(|o| o.len() == 2)
            .ok_or("evidence 必须是 start/end 原文片段编号对象或逐字引文")?;
        let start = object
            .get("start")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or("原文片段 start 编号无效")?;
        let end = object
            .get("end")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n >= start)
            .ok_or("原文片段 end 编号无效")?;
        let first = self.ranges.get(start - 1).ok_or("原文片段 start 越界")?;
        let last = self.ranges.get(end - 1).ok_or("原文片段 end 越界")?;
        Ok(&self.source[first.start..last.end])
    }

    pub(super) fn expand(&self, graph: &mut Value, budget: usize) -> Result<(), String> {
        let mut bytes = graph.to_string().len();
        let mut expand = |item: &mut Value| -> Result<(), String> {
            let Some(evidence) = item.get_mut("evidence") else {
                return Ok(());
            };
            if evidence.is_string() {
                return Ok(());
            }
            let quote = self.resolve(evidence)?;
            // Bound before allocating a copied quote; never let a compact set of
            // references expand into an unbounded graph or model repair prompt.
            if bytes.saturating_add(quote.len()) > budget {
                return Err("图谱证据展开后超过大小限制，请选择更精确的原文片段".into());
            }
            let encoded = Value::from(quote);
            bytes = bytes.saturating_add(encoded.to_string().len());
            if bytes > budget {
                return Err("图谱证据展开后超过大小限制".into());
            }
            *evidence = encoded;
            Ok(())
        };
        for node in graph
            .get_mut("nodes")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            expand(node)?;
            for alias in node
                .get_mut("aliases")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                expand(alias)?;
            }
        }
        for edge in graph
            .get_mut("edges")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            expand(edge)?;
        }
        Ok(())
    }
}
