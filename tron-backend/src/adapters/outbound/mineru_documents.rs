//! MinerU 4 V1 transport and bounded docvortex.middle v2 normalization.
use crate::{
    application::knowledge_base::{
        KnowledgeBaseError,
        local_documents::MAX_EXPANDED_BYTES,
        parsing::{
            KnowledgeDocumentParser, OcrMode, ParsedAsset, ParsingCapabilities, ParsingTier,
            StructuredParse, normalize_middle,
        },
    },
    config::MineruConfig,
};
use async_trait::async_trait;
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{Cursor, Read},
    time::Duration,
};
use tokio::time::{Instant, sleep};
mod routing;
mod runtime;
pub use runtime::RuntimeMineruDocumentParser;

#[derive(Clone)]
pub struct MineruKnowledgeDocumentParser {
    config: MineruConfig,
    client: Client,
}
fn invalid(e: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Invalid(e.to_string())
}
impl MineruKnowledgeDocumentParser {
    pub fn new(config: MineruConfig) -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("MinerU HTTP client");
        Self { config, client }
    }
    fn url(&self, path: &str) -> Result<Url, KnowledgeBaseError> {
        if !self.config.enabled {
            return Err(invalid("MinerU解析已禁用，请检查平台设置"));
        }
        let base = Url::parse(&self.config.base_url).map_err(invalid)?;
        if !matches!(base.scheme(), "http" | "https")
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(invalid("MinerU服务地址无效"));
        }
        Url::parse(&format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            path
        ))
        .map_err(invalid)
    }
    async fn bytes(&self, request: reqwest::RequestBuilder) -> Result<Vec<u8>, KnowledgeBaseError> {
        let response = request
            .send()
            .await
            .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
        self.response_bytes(response).await
    }
    async fn response_bytes(
        &self,
        mut response: reqwest::Response,
    ) -> Result<Vec<u8>, KnowledgeBaseError> {
        if !response.status().is_success() {
            let message = format!(
                "MinerU返回HTTP {}，资料解析未完成",
                response.status().as_u16()
            );
            return Err(
                if response.status().is_server_error() || response.status().as_u16() == 429 {
                    KnowledgeBaseError::Storage(message)
                } else {
                    invalid(message)
                },
            );
        }
        if response
            .content_length()
            .is_some_and(|s| s > MAX_EXPANDED_BYTES as u64)
        {
            return Err(invalid("MinerU响应超过100 MiB上限"));
        }
        let mut result = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?
        {
            if result.len().saturating_add(chunk.len()) > MAX_EXPANDED_BYTES {
                return Err(invalid("MinerU响应超过100 MiB上限"));
            }
            result.extend_from_slice(&chunk);
        }
        Ok(result)
    }
    async fn get(&self, path: &str) -> Result<Value, KnowledgeBaseError> {
        serde_json::from_slice(&self.bytes(self.client.get(self.url(path)?)).await?)
            .map_err(invalid)
    }
    async fn post(&self, path: &str, value: Value) -> Result<Value, KnowledgeBaseError> {
        serde_json::from_slice(
            &self
                .bytes(self.client.post(self.url(path)?).json(&value))
                .await?,
        )
        .map_err(invalid)
    }
    async fn submit_job(&self, value: Value) -> Result<Value, KnowledgeBaseError> {
        let response = self
            .client
            .post(self.url("/v1/parse/jobs")?)
            .json(&value)
            .send()
            .await
            .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
        if !response.status().is_success() {
            return self.response_bytes(response).await.map(|_| Value::Null);
        }
        // Missing/truncated JSON or an invalid ID may still mean a created job.
        // Keep reconciling instead of sending a second POST.
        let bytes = self
            .response_bytes(response)
            .await
            .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
        let job: Value = serde_json::from_slice(&bytes)
            .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
        Self::id(&job, "job_id").map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
        Ok(job)
    }
    fn id<'a>(v: &'a Value, key: &str) -> Result<&'a str, KnowledgeBaseError> {
        let id = v
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("MinerU缺少{key}")))?;
        if id.is_empty()
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(invalid("MinerU返回非法标识"));
        }
        Ok(id)
    }
}
#[async_trait]
impl KnowledgeDocumentParser for MineruKnowledgeDocumentParser {
    fn with_ocr_mode(
        &self,
        mode: OcrMode,
    ) -> Result<std::sync::Arc<dyn KnowledgeDocumentParser>, KnowledgeBaseError> {
        let mut parser = self.clone();
        parser.config.ocr_mode = mode.as_str().into();
        Ok(std::sync::Arc::new(parser))
    }
    fn profile_identity(&self) -> Value {
        json!({"ocrMode":self.config.ocr_mode,"parserProfile":self.config.parser_profile,"expectedParserVersion":self.config.expected_parser_version,"baseUrl":self.config.base_url,"nodes":self.config.nodes,"runtimeConfig":self.config})
    }
    fn endpoint_ids(&self) -> Vec<String> {
        self.nodes()
            .unwrap_or_default()
            .into_iter()
            .map(|n| n.id)
            .collect()
    }
    async fn healthy_endpoints(
        &self,
    ) -> Result<
        Vec<crate::application::knowledge_base::ingestion::ParseEndpointCapacity>,
        KnowledgeBaseError,
    > {
        self.routing_capacities().await
    }
    async fn routing_status(
        &self,
    ) -> crate::application::knowledge_base::parsing::ParserRoutingStatus {
        self.routing_status_with_reasons().await
    }
    async fn parse_durable(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
        checkpoint: std::sync::Arc<
            dyn crate::application::knowledge_base::parsing::ParseCheckpoint,
        >,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        let task = checkpoint.task()?;
        let parser = self.on_endpoint(
            task.endpoint_id
                .as_deref()
                .ok_or_else(|| invalid("解析节点未认领"))?,
        )?;
        parser
            .parse_internal(filename, bytes, tier, Some(checkpoint))
            .await
    }
    async fn cancel_job(&self, endpoint: &str, job: &str) -> Result<(), KnowledgeBaseError> {
        let parser = self.on_endpoint(endpoint)?;
        Self::id(&json!({"id":job}), "id")?;
        let response = parser
            .client
            .delete(parser.url(&format!("/v1/parse/jobs/{job}"))?)
            .send()
            .await
            .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
        if response.status().is_success() || matches!(response.status().as_u16(), 404 | 409) {
            Ok(())
        } else {
            Err(KnowledgeBaseError::Storage(format!(
                "MinerU取消失败HTTP {}",
                response.status()
            )))
        }
    }
    async fn cancel_task(
        &self,
        task: &crate::application::knowledge_base::ingestion::IngestionTask,
    ) -> Result<Option<String>, KnowledgeBaseError> {
        // Use the authorized snapshot even if the node is disabled later.
        // Cancellation never uploads a new source or moves to another node.
        let mut snapshot = self.clone();
        if let Some(mode) = task.configuration["parser"]["ocrMode"].as_str() {
            snapshot.config.ocr_mode = OcrMode::from_wire(mode)
                .ok_or_else(|| invalid("解析任务OCR模式无效"))?
                .as_str()
                .into();
        }
        if let Some(base) = task.configuration["parser"]["baseUrl"].as_str() {
            snapshot.config.base_url = base.into();
        }
        if let Some(nodes) = task.configuration["parser"].get("nodes") {
            snapshot.config.nodes = serde_json::from_value(nodes.clone()).map_err(invalid)?;
        }
        let endpoint = task
            .endpoint_id
            .as_deref()
            .ok_or_else(|| invalid("原解析节点身份缺失"))?;
        let parser = snapshot.on_endpoint(endpoint)?;
        let job = if let Some(job) = &task.remote_job_id {
            job.clone()
        } else if task.reconciling {
            let file = task
                .remote_file_id
                .as_deref()
                .ok_or_else(|| invalid("原解析文件身份缺失"))?;
            let tier = serde_json::from_value(task.configuration["parsingTier"].clone())
                .map_err(invalid)?;
            Self::id(&parser.reconcile_job(file, tier).await?, "job_id")?.to_owned()
        } else {
            return Ok(None);
        };
        parser.cancel_job("default", &job).await?;
        Ok(Some(job))
    }
    fn supports_local_visual_reparse(&self) -> bool {
        Url::parse(&self.config.base_url).is_ok_and(|url| {
            matches!(
                url.host_str(),
                Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
            )
        })
    }
    fn default_tier(&self) -> ParsingTier {
        self.config.default_tier
    }
    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        if !self.config.nodes.is_empty() {
            return self.pool_capabilities().await;
        }
        let health = self.get("/v1/health").await?;
        if health["status"] != "ok" {
            return Err(invalid("MinerU服务未就绪"));
        }
        let version = health["version"]
            .as_str()
            .ok_or_else(|| invalid("MinerU未报告版本"))?;
        if version != self.config.expected_parser_version {
            return Err(invalid("MinerU版本与固定解析配置不匹配"));
        }
        if !version.starts_with("4.") {
            return Err(invalid("知识库需要MinerU 4 V1服务"));
        }
        let tiers = self.get("/v1/tiers").await?;
        let available = tiers["data"]
            .as_array()
            .ok_or_else(|| invalid("MinerU未报告解析能力"))?
            .iter()
            .filter_map(|v| serde_json::from_value(v["id"].clone()).ok())
            .collect::<Vec<ParsingTier>>();
        if !available.contains(&self.config.default_tier) {
            return Err(invalid("MinerU不支持平台默认解析质量"));
        }
        Ok(ParsingCapabilities {
            default_tier: self.config.default_tier,
            default_ocr_mode: OcrMode::from_wire(&self.config.ocr_mode)
                .ok_or_else(|| invalid("MinerU OCR模式无效"))?,
            available_tiers: available,
            parser_version: version.into(),
            office_tier: ParsingTier::Flash,
        })
    }
    async fn parse(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        if !self.config.nodes.is_empty() {
            let capacities = self.routing_capacities().await?;
            let first = capacities
                .first()
                .ok_or_else(|| invalid("没有可用解析节点"))?;
            return self
                .on_endpoint(&first.endpoint_id)?
                .parse_internal(filename, bytes, tier, None)
                .await;
        }
        self.parse_internal(filename, bytes, tier, None).await
    }
}
impl MineruKnowledgeDocumentParser {
    async fn parse_internal(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
        checkpoint: Option<
            std::sync::Arc<dyn crate::application::knowledge_base::parsing::ParseCheckpoint>,
        >,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        if !matches!(self.config.ocr_mode.as_str(), "auto" | "txt" | "ocr") {
            return Err(invalid("MinerU OCR模式无效"));
        }
        if bytes.len() > self.config.max_bytes.min(20 * 1024 * 1024) {
            return Err(invalid("资料超过解析大小上限"));
        }
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let office = matches!(extension.as_str(), "docx" | "xlsx");
        if !office && extension != "pdf" {
            return Err(invalid("MinerU适配仅处理.docx/.xlsx/.pdf"));
        }
        let selected = if office {
            if tier.is_some_and(|t| t != ParsingTier::Flash) {
                return Err(invalid(
                    "Word/Excel使用原生解析，请选择继承平台或原生文本解析",
                ));
            }
            ParsingTier::Flash
        } else {
            tier.unwrap_or(self.config.default_tier)
        };
        let mut probe = self.clone();
        probe.config.default_tier = selected;
        let capabilities = probe.capabilities().await?;
        if !capabilities.available_tiers.contains(&selected) {
            return Err(invalid("MinerU当前不支持选定解析质量，不会自动降级"));
        }
        let saved = checkpoint.as_ref().map(|c| c.task()).transpose()?;
        let reusable = if let Some(task) = &saved {
            if let Some(id) = &task.remote_job_id {
                let response = self
                    .client
                    .get(self.url(&format!("/v1/parse/jobs/{id}"))?)
                    .send()
                    .await
                    .map_err(|e| KnowledgeBaseError::Storage(e.to_string()))?;
                if response.status().as_u16() == 404 {
                    checkpoint.as_ref().unwrap().save(None, None, false)?;
                    None
                } else {
                    let job: Value = serde_json::from_slice(&self.response_bytes(response).await?)
                        .map_err(invalid)?;
                    if matches!(
                        job["status"].as_str(),
                        Some("failed" | "partial" | "canceled")
                    ) {
                        checkpoint.as_ref().unwrap().save(
                            task.remote_file_id.clone(),
                            None,
                            false,
                        )?;
                        None
                    } else {
                        Some(job)
                    }
                }
            } else if task.reconciling {
                Some(
                    self.reconcile_job(
                        task.remote_file_id
                            .as_deref()
                            .ok_or_else(|| invalid("MinerU对账文件身份缺失"))?,
                        selected,
                    )
                    .await?,
                )
            } else {
                None
            }
        // Only the durable Tron task can prove which OCR mode created a job.
        // MinerU's job response does not include the requested OCR mode.
        } else {
            None
        };
        let resumed = reusable.is_some();
        let mut job = if let Some(job) = reusable {
            if let Some(checkpoint) = &checkpoint {
                let task = checkpoint.task()?;
                checkpoint.save(
                    task.remote_file_id,
                    Some(Self::id(&job, "job_id")?.into()),
                    false,
                )?;
            }
            job
        } else {
            let current = checkpoint.as_ref().map(|c| c.task()).transpose()?;
            let file_id =
                if let Some(file) = current.as_ref().and_then(|t| t.remote_file_id.clone()) {
                    file
                } else {
                    let file = self.upload_source(filename, bytes, &extension).await?;
                    if let Some(checkpoint) = &checkpoint {
                        checkpoint.save(Some(file.clone()), None, false)?;
                    }
                    file
                };
            if let Some(checkpoint) = &checkpoint {
                checkpoint.save(Some(file_id.clone()), None, true)?;
            }
            let created = self.submit_job(json!({"files":[{"source":{"type":"file_id","file_id":file_id}}],"tier":selected,"ocr_mode":self.config.ocr_mode,"output_formats":["zip","structured_content"]})).await;
            if matches!(&created, Err(KnowledgeBaseError::Invalid(_))) {
                if let Some(checkpoint) = &checkpoint {
                    checkpoint.save(Some(file_id.clone()), None, false)?;
                }
            }
            let job = created?;
            if let Some(checkpoint) = &checkpoint {
                let id = Self::id(&job, "job_id")?.to_owned();
                if let Err(error) = checkpoint.save(Some(file_id), Some(id.clone()), false) {
                    let _ = self
                        .client
                        .delete(self.url(&format!("/v1/parse/jobs/{id}"))?)
                        .send()
                        .await;
                    return Err(error);
                }
            }
            job
        };
        let job_id = Self::id(&job, "job_id")?.to_owned();
        let deadline = Instant::now() + Duration::from_secs(self.config.timeout_seconds);
        loop {
            match job["status"].as_str() {
                Some("completed") => break,
                Some("queued" | "running") => {
                    if Instant::now() >= deadline {
                        return Err(KnowledgeBaseError::Storage(format!(
                            "MinerU解析超时，可重试（任务{job_id}）"
                        )));
                    }
                    sleep(Duration::from_millis(750)).await;
                    job = self.get(&format!("/v1/parse/jobs/{job_id}")).await?;
                }
                Some("failed" | "partial" | "canceled") => {
                    return Err(invalid(parse_failure(&job, &job_id)));
                }
                _ => return Err(invalid("MinerU返回未知任务状态")),
            }
        }
        let files = job["files"]
            .as_array()
            .ok_or_else(|| invalid("MinerU缺少解析输出"))?;
        if files.len() != 1 {
            return Err(invalid("MinerU返回了不匹配的资料输出"));
        }
        if (resumed || checkpoint.is_some())
            && files[0]["parse"]["parser_version"] != capabilities.parser_version
        {
            return Err(invalid("MinerU复用任务的解析版本发生变化，拒绝发布"));
        }
        let output = &files[0]["output_files"];
        let zip_id = Self::id(&output["zip"], "file_id")?;
        let archive = self
            .bytes(
                self.client
                    .get(self.url(&format!("/v1/files/{zip_id}/content"))?),
            )
            .await?;
        decode_archive(&archive, selected, &capabilities.parser_version, &job_id)
    }
}

fn parse_failure(job: &Value, job_id: &str) -> String {
    let error_message = |error: &Value| {
        error["message"]
            .as_str()
            .or_else(|| error.as_str())
            .filter(|message| !message.trim().is_empty())
            .map(str::to_owned)
    };
    let reason = job["files"]
        .as_array()
        .and_then(|files| files.iter().find_map(|file| error_message(&file["error"])))
        .or_else(|| error_message(&job["error"]));
    // Keep remote diagnostics bounded and render them as plain text in the UI.
    let reason = reason.map(|message| {
        message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(1000)
            .collect::<String>()
    });
    format!(
        "MINERU_PARSE_FAILED:{}",
        json!({"jobId": job_id, "reason": reason})
    )
}

pub fn decode_archive(
    bytes: &[u8],
    tier: ParsingTier,
    version: &str,
    job: &str,
) -> Result<StructuredParse, KnowledgeBaseError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(invalid)?;
    if archive.len() > 4096 {
        return Err(invalid("MinerU输出文件过多"));
    }
    let mut expanded = 0usize;
    let mut middle = None;
    let mut structured = None;
    let mut markdown = None;
    let mut assets = Vec::new();
    let mut paths = BTreeSet::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(invalid)?;
        let path = file
            .enclosed_name()
            .ok_or_else(|| invalid("MinerU输出含非法路径"))?;
        let name = path
            .to_str()
            .ok_or_else(|| invalid("MinerU输出路径无效"))?
            .replace('\\', "/");
        if file.is_dir() {
            continue;
        }
        if !paths.insert(name.clone()) {
            return Err(invalid("MinerU输出含重复文件"));
        }
        let remaining = MAX_EXPANDED_BYTES.saturating_sub(expanded);
        if file.size() > remaining as u64 {
            return Err(invalid("MinerU解压内容超过100 MiB上限"));
        }
        let mut data = Vec::new();
        file.by_ref()
            .take(remaining as u64 + 1)
            .read_to_end(&mut data)
            .map_err(invalid)?;
        expanded = expanded.saturating_add(data.len());
        if expanded > MAX_EXPANDED_BYTES {
            return Err(invalid("MinerU解压内容超过100 MiB上限"));
        }
        match name.as_str() {
            "middle_json.json" => {
                middle = Some(serde_json::from_slice::<Value>(&data).map_err(invalid)?)
            }
            "structured_content.json" => {
                structured = Some(serde_json::from_slice::<Value>(&data).map_err(invalid)?)
            }
            "markdown.md" => markdown = Some(String::from_utf8(data).map_err(invalid)?),
            _ if name.starts_with("images/") => {
                let extension = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                    return Err(invalid("MinerU图片附件格式不受支持"));
                }
                assets.push(ParsedAsset {
                    path: name,
                    bytes: data,
                });
            }
            _ => {}
        }
    }
    let middle = middle.ok_or_else(|| invalid("MinerU缺少结构化中间结果"))?;
    if middle["schema"] != "docvortex.middle" || middle["schema_version"] != "2.0" {
        return Err(invalid("MinerU结构化结果版本不受支持"));
    }
    if middle["extensions"]["mineru"]["tier"] != tier.as_str() {
        return Err(invalid("MinerU实际解析质量与请求不同，拒绝发布"));
    }
    let paths = assets.iter().map(|a| a.path.clone()).collect();
    let document = normalize_middle(&middle, &paths)?;
    Ok(StructuredParse {
        document,
        middle,
        structured: structured.ok_or_else(|| invalid("MinerU缺少structured_content"))?,
        markdown: markdown.ok_or_else(|| invalid("MinerU缺少Markdown输出"))?,
        assets,
        effective_tier: tier,
        parser_version: version.into(),
        vendor_job_id: job.into(),
    })
}
