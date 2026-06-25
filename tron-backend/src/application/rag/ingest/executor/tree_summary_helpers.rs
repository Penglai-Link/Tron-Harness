#[derive(Debug, Clone)]
struct TreeExtractedNode {
    tree_node_code: String,
    parent_node_code: String,
    path_key: String,
    level: i64,
    ordinal: i64,
    title: String,
    covered_text_unit_codes: Vec<String>,
}

fn tree_extracted_nodes(tree_extracted: &Value) -> Result<Vec<TreeExtractedNode>, RagError> {
    tree_extracted
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestStageFailed)?
        .iter()
        .map(tree_extracted_node)
        .collect()
}

fn tree_extracted_node(node: &Value) -> Result<TreeExtractedNode, RagError> {
    Ok(TreeExtractedNode {
        tree_node_code: required_string(node, "treeNodeCode")?,
        parent_node_code: required_string(node, "parentNodeCode")?,
        path_key: required_string(node, "pathKey")?,
        level: required_i64(node, "level")?,
        ordinal: required_i64(node, "ordinal")?,
        title: required_string(node, "title")?,
        covered_text_unit_codes: required_string_array(node, "coveredTextUnitCodes")?,
    })
}

fn tree_parent_child_edges<'a>(
    tree_extracted: &Value,
    tree_nodes: &'a [TreeExtractedNode],
) -> Result<BTreeMap<String, Vec<&'a TreeExtractedNode>>, RagError> {
    let mut nodes_by_code = BTreeMap::new();
    for tree_node in tree_nodes {
        if nodes_by_code
            .insert(tree_node.tree_node_code.as_str(), tree_node)
            .is_some()
        {
            return Err(RagError::IngestStageFailed);
        }
    }

    let mut children_by_parent: BTreeMap<String, Vec<&TreeExtractedNode>> = BTreeMap::new();
    let mut linked_child_codes = BTreeSet::new();
    for edge in tree_extracted
        .get("edges")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestStageFailed)?
    {
        if required_string(edge, "relationship")? != "PARENT_OF" {
            continue;
        }
        let parent_code = required_string(edge, "from")?;
        let child_code = required_string(edge, "to")?;
        let child = *nodes_by_code
            .get(child_code.as_str())
            .ok_or(RagError::IngestStageFailed)?;
        if child.parent_node_code != parent_code {
            return Err(RagError::IngestStageFailed);
        }
        children_by_parent
            .entry(parent_code)
            .or_default()
            .push(child);
        linked_child_codes.insert(child_code);
    }
    for tree_node in tree_nodes
        .iter()
        .filter(|tree_node| tree_node.parent_node_code != "0")
    {
        if !linked_child_codes.contains(&tree_node.tree_node_code) {
            return Err(RagError::IngestStageFailed);
        }
    }
    for children in children_by_parent.values_mut() {
        children.sort_by_key(|tree_node| tree_node.ordinal);
    }
    Ok(children_by_parent)
}

fn required_string(value: &Value, field: &str) -> Result<String, RagError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(RagError::IngestStageFailed)
}

fn required_i64(value: &Value, field: &str) -> Result<i64, RagError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or(RagError::IngestStageFailed)
}

fn required_string_array(value: &Value, field: &str) -> Result<Vec<String>, RagError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or(RagError::IngestStageFailed)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or(RagError::IngestStageFailed)
        })
        .collect()
}

fn tree_summary_post_order<'a>(
    tree_nodes: &'a [TreeExtractedNode],
    children_by_parent: &BTreeMap<String, Vec<&'a TreeExtractedNode>>,
) -> Result<Vec<&'a TreeExtractedNode>, RagError> {
    let mut roots = tree_nodes
        .iter()
        .filter(|tree_node| tree_node.parent_node_code == "0")
        .collect::<Vec<_>>();
    roots.sort_by_key(|tree_node| tree_node.ordinal);

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::with_capacity(tree_nodes.len());
    for root in roots {
        append_tree_summary_post_order(
            root,
            children_by_parent,
            &mut visiting,
            &mut visited,
            &mut ordered,
        )?;
    }

    let mut unvisited = tree_nodes
        .iter()
        .filter(|tree_node| !visited.contains(&tree_node.tree_node_code))
        .collect::<Vec<_>>();
    unvisited.sort_by_key(|tree_node| tree_node.ordinal);
    for tree_node in unvisited {
        append_tree_summary_post_order(
            tree_node,
            children_by_parent,
            &mut visiting,
            &mut visited,
            &mut ordered,
        )?;
    }

    if ordered.len() != tree_nodes.len() {
        return Err(RagError::IngestStageFailed);
    }
    Ok(ordered)
}

fn append_tree_summary_post_order<'a>(
    tree_node: &'a TreeExtractedNode,
    children_by_parent: &BTreeMap<String, Vec<&'a TreeExtractedNode>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    ordered: &mut Vec<&'a TreeExtractedNode>,
) -> Result<(), RagError> {
    if visited.contains(&tree_node.tree_node_code) {
        return Ok(());
    }
    if !visiting.insert(tree_node.tree_node_code.clone()) {
        return Err(RagError::IngestStageFailed);
    }
    if let Some(children) = children_by_parent.get(&tree_node.tree_node_code) {
        for child in children {
            append_tree_summary_post_order(child, children_by_parent, visiting, visited, ordered)?;
        }
    }
    visiting.remove(&tree_node.tree_node_code);
    visited.insert(tree_node.tree_node_code.clone());
    ordered.push(tree_node);
    Ok(())
}

fn direct_text_units(
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    children_by_parent: &BTreeMap<String, Vec<&TreeExtractedNode>>,
) -> Vec<String> {
    let child_covered_codes = children_by_parent
        .get(&tree_node.tree_node_code)
        .into_iter()
        .flat_map(|children| children.iter())
        .flat_map(|child| child.covered_text_unit_codes.iter().cloned())
        .collect::<BTreeSet<_>>();
    prepared
        .units
        .iter()
        .filter(|unit| {
            tree_node
                .covered_text_unit_codes
                .contains(&unit.text_unit_code)
        })
        .filter(|unit| !child_covered_codes.contains(&unit.text_unit_code))
        .map(|unit| unit.text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect()
}

fn llm_tree_summary_prompt(
    title: &str,
    child_summaries: &[(&str, &str)],
    direct_text_units: &[String],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("Summarize this RAG tree node. Return only the summary text.\n");
    prompt.push_str("Node title:\n");
    prompt.push_str(title.trim());
    prompt.push('\n');
    if !child_summaries.is_empty() {
        prompt.push_str("\nChild summaries:\n");
        for (child_title, child_summary) in child_summaries {
            prompt.push_str("- ");
            prompt.push_str(child_title.trim());
            prompt.push_str(": ");
            prompt.push_str(child_summary.trim());
            prompt.push('\n');
        }
    }
    if !direct_text_units.is_empty() {
        prompt.push_str("\nDirect text units:\n");
        for (index, text) in direct_text_units.iter().enumerate() {
            prompt.push_str("[");
            prompt.push_str(&(index + 1).to_string());
            prompt.push_str("]\n");
            prompt.push_str(text);
            prompt.push('\n');
        }
    }
    prompt
}

fn llm_summary_tree_node(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    summary_text: String,
    model_code: String,
) -> RagTreeNodeWrite {
    summary_tree_node_write(
        request,
        prepared,
        tree_node,
        summary_text,
        json!({
            "summaryStrategy": "RAG_TREE_LLM_SUMMARY_V1",
            "summaryModelCode": model_code,
            "summaryStatus": "SUMMARIZED"
        }),
    )
}

fn extractive_summary_tree_node(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
) -> RagTreeNodeWrite {
    let covered_text = prepared
        .units
        .iter()
        .filter(|unit| {
            tree_node
                .covered_text_unit_codes
                .contains(&unit.text_unit_code)
        })
        .map(|unit| unit.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let summary_text = extractive_summary(&tree_node.title, &covered_text);
    summary_tree_node_write(
        request,
        prepared,
        tree_node,
        summary_text,
        json!({ "summaryStrategy": "EXTRACTIVE_DETERMINISTIC_V1" }),
    )
}

fn summary_tree_node_write(
    request: &RagIngestRequest,
    prepared: &PreparedIngestPayload,
    tree_node: &TreeExtractedNode,
    summary_text: String,
    metadata: Value,
) -> RagTreeNodeWrite {
    let covered_ordinals = prepared
        .units
        .iter()
        .filter(|unit| {
            tree_node
                .covered_text_unit_codes
                .contains(&unit.text_unit_code)
        })
        .map(|unit| unit.ordinal as i64)
        .collect::<Vec<_>>();
    RagTreeNodeWrite {
        tree_node_code: tree_node.tree_node_code.clone(),
        parent_node_code: tree_node.parent_node_code.clone(),
        source_code: request.source.code.clone(),
        source_version: request.source.version.clone(),
        scope_type: request.target_scope.scope_type.clone(),
        scope_code: request.target_scope.scope_code.clone(),
        path_key: tree_node.path_key.clone(),
        level: tree_node.level,
        ordinal: tree_node.ordinal,
        title: tree_node.title.clone(),
        summary_token_count: token_count(&summary_text),
        summary_content_hash: stable_hash(&summary_text),
        summary_text,
        summary_fulltext_status: "PENDING".into(),
        summary_embedding_status: "PENDING".into(),
        summary_embedding_model_code: None,
        summary_embedding_version: None,
        summary_embedding_text_hash: None,
        summary_embedding_dimension: None,
        summary_embedding: None,
        covered_text_unit_count: tree_node.covered_text_unit_codes.len() as i64,
        covered_text_unit_codes: tree_node.covered_text_unit_codes.clone(),
        child_start_ordinal: covered_ordinals.iter().min().copied().unwrap_or(0),
        child_end_ordinal: covered_ordinals.iter().max().copied().unwrap_or(0),
        metadata,
        status: "ACTIVE".into(),
    }
}

#[cfg(test)]
mod tree_stage_tests {
    use super::*;

    #[test]
    fn tree_summary_post_order_uses_edges_even_when_levels_are_inconsistent() {
        let nodes = vec![
            extracted_node("root", "0", 99, 0, "Root", &["u0", "u1"]),
            extracted_node("parent", "root", 1, 1, "Parent", &["u0", "u1"]),
            extracted_node("leaf", "parent", 0, 2, "Leaf", &["u1"]),
        ];
        let mut children_by_parent = BTreeMap::new();
        children_by_parent.insert("root".into(), vec![&nodes[1]]);
        children_by_parent.insert("parent".into(), vec![&nodes[2]]);

        let ordered = tree_summary_post_order(&nodes, &children_by_parent).unwrap();

        assert_eq!(
            ordered
                .iter()
                .map(|node| node.tree_node_code.as_str())
                .collect::<Vec<_>>(),
            vec!["leaf", "parent", "root"]
        );
    }

    fn extracted_node(
        tree_node_code: &str,
        parent_node_code: &str,
        level: i64,
        ordinal: i64,
        title: &str,
        covered_text_unit_codes: &[&str],
    ) -> TreeExtractedNode {
        TreeExtractedNode {
            tree_node_code: tree_node_code.into(),
            parent_node_code: parent_node_code.into(),
            path_key: title.into(),
            level,
            ordinal,
            title: title.into(),
            covered_text_unit_codes: covered_text_unit_codes
                .iter()
                .map(|code| (*code).into())
                .collect(),
        }
    }
}
