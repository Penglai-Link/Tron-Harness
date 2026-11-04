//! Person-AI document parsing transport. The platform owns submission, billing,
//! and result files; Tron owns the reusable parse artifact and RAG index.
use crate::application::{
    knowledge_base::{
        KnowledgeBaseError,
        ingestion::{IngestionTask, ParseEndpointCapacity},
        local_documents::{
            MAX_EXPANDED_BYTES,
        },
        parsing::{
            KnowledgeDocumentParser, OcrMode, ParseCheckpoint, ParsedAsset, ParsingCapabilities,
            ParsingTier, StructuredParse,
        },
    },
    model_provider::ModelProviderService,
};
use async_trait::async_trait;
use crab_serverapi::document_parse::{CreateParseTask, DocumentParseClient, ParseMode, ParseTask};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};

mod lifecycle;
use lifecycle::{ParseSubmission, TaskProgress};

const PARSER_VERSION: &str = "person-ai-document-parse-v1";
const POLL_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct PersonAiDocumentParser {
    providers: ModelProviderService,
    ocr_mode: OcrMode,
}

impl PersonAiDocumentParser {
    pub fn new(providers: ModelProviderService) -> Self {
        Self {
            providers,
            ocr_mode: OcrMode::Auto,
        }
    }

    fn mode(&self, tier: Option<ParsingTier>) -> ParseMode {
        use eliteforge_agent_capability_personai::{PersonAiDocumentParsingIntent as Intent, personai_document_parse_mode};
        personai_document_parse_mode(if tier == Some(ParsingTier::Advanced) {
            Intent::Layout
        } else if self.ocr_mode == OcrMode::Ocr {
            Intent::Recognition
        } else {
            Intent::Native
        })
    }

    async fn run(
        &self,
        user: &str,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
        checkpoint: Option<Arc<dyn ParseCheckpoint>>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        if bytes.is_empty() || bytes.len() > 50 * 1024 * 1024 {
            return Err(KnowledgeBaseError::Invalid(
                "文件必须非空且不超过 50 MB".into(),
            ));
        }
        let mode = self.mode(tier);
        let prior = checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.task())
            .transpose()?;
        let mut submission = ParseSubmission::new(filename, bytes, mode, prior.as_ref());
        let (credentials, base_url) =
            self.providers
                .document_parse_session(user)
                .await
                .map_err(|error| {
                    KnowledgeBaseError::Storage(format!("Person-AI 解析账号暂不可用：{error}"))
                })?;
        let context = parse_context(credentials);
        let client = DocumentParseClient::new(base_url).map_err(platform_error)?;
        let task = submission
            .load(&client, &context, checkpoint.as_ref())
            .await?;
        let task = self
            .wait_for_settlement(user, task, checkpoint.as_ref(), &mut submission)
            .await?;
        self.result(user, filename, bytes, tier, task).await
    }

    async fn wait_for_settlement(
        &self,
        user: &str,
        mut task: ParseTask,
        checkpoint: Option<&Arc<dyn ParseCheckpoint>>,
        submission: &mut ParseSubmission<'_>,
    ) -> Result<ParseTask, KnowledgeBaseError> {
        loop {
            match lifecycle::progress(&task)? {
                TaskProgress::Ready => return Ok(task),
                TaskProgress::Replace => {
                    let (credentials, base_url) = self
                        .providers
                        .document_parse_session(user)
                        .await
                        .map_err(platform_error)?;
                    let context = parse_context(credentials);
                    let client = DocumentParseClient::new(base_url).map_err(platform_error)?;
                    task = submission.replace(&client, &context, checkpoint).await?;
                    continue;
                }
                TaskProgress::Wait => {}
            }
            if let Some(checkpoint) = checkpoint {
                checkpoint.task()?;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
            let (credentials, base_url) = self
                .providers
                .document_parse_session(user)
                .await
                .map_err(|error| {
                    KnowledgeBaseError::Storage(format!("Person-AI 解析账号暂不可用：{error}"))
                })?;
            let context = parse_context(credentials);
            let client = DocumentParseClient::new(base_url).map_err(platform_error)?;
            task = submission
                .poll(&client, &context, &task.id, checkpoint)
                .await?;
        }
    }

    async fn result(
        &self,
        user: &str,
        filename: &str,
        source: &[u8],
        tier: Option<ParsingTier>,
        task: ParseTask,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        let result = lifecycle::result_manifest(&task)?;
        let files = result["files"]
            .as_array()
            .ok_or_else(|| KnowledgeBaseError::Storage("平台解析产物缺少文件列表".into()))?;
        let file = files
            .iter()
            .find(|file| file["filename"] == filename)
            .or_else(|| (files.len() == 1).then(|| &files[0]))
            .ok_or_else(|| KnowledgeBaseError::Storage("解析结果文件不匹配".into()))?;
        let source_hash = format!("{:x}", Sha256::digest(source));
        if file["source_hash"]
            .as_str()
            .is_some_and(|hash| hash != source_hash)
        {
            return Err(KnowledgeBaseError::Storage(
                "解析结果与原始文件哈希不一致".into(),
            ));
        }
        if file["successful_pages"].as_i64().unwrap_or(0) <= 0 {
            return Err(KnowledgeBaseError::Invalid("文档没有成功解析的页面".into()));
        }
        if file["total_pages"]
            .as_i64()
            .is_some_and(|total| total > file["successful_pages"].as_i64().unwrap_or(0))
        {
            return Err(KnowledgeBaseError::Invalid(
                "文档有未成功解析的页面，未发布不完整索引".into(),
            ));
        }
        let markdown_id = output_id(file, "markdown")?;
        let middle_id = output_id(file, "middle_json")?;
        let (credentials, base_url) =
            self.providers
                .document_parse_session(user)
                .await
                .map_err(|error| {
                    KnowledgeBaseError::Storage(format!("Person-AI 解析账号暂不可用：{error}"))
                })?;
        let context = parse_context(credentials);
        let client = DocumentParseClient::new(base_url).map_err(platform_error)?;
        // Supplier Markdown may include large inline image data. The artifact
        // budget bounds those bytes; the separate text budget applies after
        // image payloads are removed, before chunking/embedding.
        let markdown = download(&client, &context, &task.id, markdown_id, MAX_EXPANDED_BYTES).await?;
        let middle = download(&client, &context, &task.id, middle_id, MAX_EXPANDED_BYTES).await?;
        let assets = if let Ok(zip_id) = output_id(file, "zip") {
            let archive = download_to_temp(&client, &context, &task.id, zip_id).await?;
            images_from_result_zip(&archive)?
        } else {
            Vec::new()
        };
        let mut middle: Value = serde_json::from_slice(&middle)
            .map_err(|_| KnowledgeBaseError::Storage("平台解析结构化结果无效".into()))?;
        let markdown_text = String::from_utf8(markdown)
            .map_err(|_| KnowledgeBaseError::Storage("平台 Markdown 结果不是 UTF-8".into()))?;
        attach_segment_source_ranges(&mut middle, file)?;
        let paths = assets.iter().map(|asset| asset.path.clone()).collect();
        let mut document = crate::application::knowledge_base::parsing::normalize_parse_middle(&middle, &paths)?;
        document.format = Path::new(filename)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("document")
            .to_ascii_lowercase();
        if !middle.is_array() {
            return Err(KnowledgeBaseError::Storage(
                "平台结构化结果格式不受支持；未发布缺少图表结构的索引".into(),
            ));
        }
        Ok(StructuredParse {
            document,
            structured: json!({"schema":"person-ai.document-parse","middleFormat":"segments"}),
            middle,
            markdown: markdown_text,
            assets,
            effective_tier: tier.unwrap_or(ParsingTier::Standard),
            parser_version: PARSER_VERSION.into(),
            vendor_job_id: task.id,
        })
    }
}

/// Person-AI declares zero-based source ranges in the settled result manifest.
/// Preserve them in the cached middle artifact; segment size is never a page offset.
fn attach_segment_source_ranges(middle: &mut Value, file: &Value) -> Result<(), KnowledgeBaseError> {
    let Some(parts) = middle.as_array_mut() else { return Ok(()); };
    let Some(segments) = file["segments"].as_array() else { return Ok(()); };
    for part in parts {
        if part["data"]["metadata"]["file_suffix"] != "pdf" { continue; }
        let seq = part["seq"].as_u64().ok_or_else(|| KnowledgeBaseError::Invalid("解析分段序号无效".into()))?;
        let matches = segments.iter().filter(|segment| segment["seq"].as_u64() == Some(seq)).collect::<Vec<_>>();
        if matches.len() != 1 { return Err(KnowledgeBaseError::Invalid("解析分段来源范围缺失或重复".into())); }
        let range = matches[0]["page_range"].as_array().filter(|range| range.len() == 2)
            .ok_or_else(|| KnowledgeBaseError::Invalid("解析分段来源范围无效".into()))?;
        let start = range[0].as_u64().ok_or_else(|| KnowledgeBaseError::Invalid("解析分段起始页无效".into()))?;
        let end = range[1].as_u64().ok_or_else(|| KnowledgeBaseError::Invalid("解析分段结束页无效".into()))?;
        let pages = part["data"]["pages"].as_array().ok_or_else(|| KnowledgeBaseError::Invalid("解析分段页信息缺失".into()))?;
        if end <= start || end - start != pages.len() as u64 || pages.iter().enumerate().any(|(i,page)|page["page_idx"].as_u64()!=Some(i as u64)) {
            return Err(KnowledgeBaseError::Invalid("解析分段与原始页范围不一致".into()));
        }
        part["source_page_start"] = start.into();
    }
    Ok(())
}

fn images_from_result_zip(
    archive: &tempfile::NamedTempFile,
) -> Result<Vec<ParsedAsset>, KnowledgeBaseError> {
    let mut outer = zip::ZipArchive::new(archive.reopen().map_err(platform_error)?)
        .map_err(|_| KnowledgeBaseError::Storage("平台解析 ZIP 无效".into()))?;
    if outer.len() > 4096 {
        return Err(KnowledgeBaseError::Invalid("解析附件数量超过上限".into()));
    }
    let mut images = BTreeMap::<String, Vec<u8>>::new();
    let mut duplicates = BTreeSet::new();
    let mut expanded = 0usize;
    for i in 0..outer.len() {
        let mut item = outer.by_index(i).map_err(platform_error)?;
        let Some(path) = item.enclosed_name() else {
            return Err(KnowledgeBaseError::Invalid("解析附件路径无效".into()));
        };
        let name = path.to_string_lossy();
        if item.is_dir() || !name.starts_with("zip/") || !name.ends_with(".zip") {
            continue;
        }
        let mut inner_file = tempfile::NamedTempFile::new().map_err(platform_error)?;
        let copied = std::io::copy(
            &mut item
                .by_ref()
                .take((MAX_EXPANDED_BYTES - expanded) as u64 + 1),
            &mut inner_file,
        )
        .map_err(platform_error)?;
        expanded = expanded.saturating_add(copied as usize);
        if expanded > MAX_EXPANDED_BYTES {
            return Err(KnowledgeBaseError::Invalid(
                "解析附件解压内容超过100 MiB".into(),
            ));
        }
        let mut inner = zip::ZipArchive::new(inner_file.reopen().map_err(platform_error)?)
            .map_err(|_| KnowledgeBaseError::Storage("平台解析图片 ZIP 无效".into()))?;
        if inner.len() > 4096 {
            return Err(KnowledgeBaseError::Invalid("解析图片数量超过上限".into()));
        }
        for j in 0..inner.len() {
            let mut image = inner.by_index(j).map_err(platform_error)?;
            let Some(path) = image.enclosed_name() else {
                return Err(KnowledgeBaseError::Invalid("解析图片路径无效".into()));
            };
            let name = path.to_string_lossy().replace('\\', "/");
            if image.is_dir() || !name.starts_with("images/") {
                continue;
            }
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            if image.size() > 20 * 1024 * 1024 {
                return Err(KnowledgeBaseError::Invalid("单张解析图片超过20 MiB".into()));
            }
            let mut data = Vec::new();
            image
                .by_ref()
                .take((MAX_EXPANDED_BYTES - expanded) as u64 + 1)
                .read_to_end(&mut data)
                .map_err(platform_error)?;
            expanded = expanded.saturating_add(data.len());
            if expanded > MAX_EXPANDED_BYTES {
                return Err(KnowledgeBaseError::Invalid(
                    "解析附件解压内容超过100 MiB".into(),
                ));
            }
            if images.insert(name.clone(), data).is_some() {
                duplicates.insert(name);
            }
        }
    }
    for name in duplicates {
        images.remove(&name);
    }
    Ok(images
        .into_iter()
        .map(|(path, bytes)| ParsedAsset { path, bytes })
        .collect())
}

async fn download_to_temp(
    client: &DocumentParseClient,
    context: &crab_serverapi::AuthorizedClientContext,
    task_id: &str,
    file_id: &str,
) -> Result<tempfile::NamedTempFile, KnowledgeBaseError> {
    let mut response = client
        .download_result(context, task_id, file_id)
        .await
        .map_err(platform_error)?;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_EXPANDED_BYTES as u64)
    {
        return Err(KnowledgeBaseError::Invalid(
            "解析 ZIP 超过100 MiB上限".into(),
        ));
    }
    let mut archive = tempfile::NamedTempFile::new().map_err(platform_error)?;
    let mut size = 0usize;
    while let Some(chunk) = response.chunk().await.map_err(platform_error)? {
        size = size.saturating_add(chunk.len());
        if size > MAX_EXPANDED_BYTES {
            return Err(KnowledgeBaseError::Invalid(
                "解析 ZIP 超过100 MiB上限".into(),
            ));
        }
        archive.write_all(&chunk).map_err(platform_error)?;
    }
    Ok(archive)
}

fn output_id<'a>(file: &'a Value, kind: &str) -> Result<&'a str, KnowledgeBaseError> {
    let output = &file["outputs"][kind];
    output
        .as_str()
        .or_else(|| output["file_id"].as_str())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| KnowledgeBaseError::Storage(format!("解析产物缺少{kind}")))
}

async fn download(
    client: &DocumentParseClient,
    context: &crab_serverapi::AuthorizedClientContext,
    task_id: &str,
    file_id: &str,
    limit: usize,
) -> Result<Vec<u8>, KnowledgeBaseError> {
    let mut response = client
        .download_result(context, task_id, file_id)
        .await
        .map_err(platform_error)?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(KnowledgeBaseError::Invalid("解析产物超过大小上限".into()));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(platform_error)? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(KnowledgeBaseError::Invalid("解析产物超过大小上限".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn platform_error(error: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Storage(format!("Person-AI 文件解析失败：{error}"))
}

fn parse_context(
    context: crab_serverapi::AuthorizedClientContext,
) -> crab_serverapi::AuthorizedClientContext {
    crab_serverapi::AuthorizedClientContext {
        token: context.token,
        refresh_token: context.refresh_token,
        client_id: context.client_id,
        public_key: context.public_key,
        product_code: context.product_code,
        product_version: context.product_version,
    }
}

#[async_trait]
impl KnowledgeDocumentParser for PersonAiDocumentParser {
    fn snapshot(&self) -> Option<Arc<dyn KnowledgeDocumentParser>> {
        Some(Arc::new(self.clone()))
    }

    fn task_snapshot(
        &self,
        identity: &Value,
    ) -> Result<Option<Arc<dyn KnowledgeDocumentParser>>, KnowledgeBaseError> {
        if identity["provider"] != "person-ai-document-parse" {
            return Err(KnowledgeBaseError::Invalid(
                "原解析任务的供应方已改变".into(),
            ));
        }
        let mode = serde_json::from_value(identity["ocrMode"].clone())
            .map_err(|_| KnowledgeBaseError::Invalid("原解析任务的文字识别模式无效".into()))?;
        Ok(Some(Arc::new(Self {
            providers: self.providers.clone(),
            ocr_mode: mode,
        })))
    }

    fn default_tier(&self) -> ParsingTier {
        ParsingTier::Standard
    }

    fn with_ocr_mode(
        &self,
        mode: OcrMode,
    ) -> Result<Arc<dyn KnowledgeDocumentParser>, KnowledgeBaseError> {
        Ok(Arc::new(Self {
            providers: self.providers.clone(),
            ocr_mode: mode,
        }))
    }

    fn profile_identity(&self) -> Value {
        json!({"provider":"person-ai-document-parse","ocrMode":self.ocr_mode,"version":PARSER_VERSION})
    }

    fn endpoint_ids(&self) -> Vec<String> {
        vec!["person-ai-document-parse".into()]
    }

    async fn healthy_endpoints(&self) -> Result<Vec<ParseEndpointCapacity>, KnowledgeBaseError> {
        Ok(vec![ParseEndpointCapacity {
            endpoint_id: "person-ai-document-parse".into(),
            capacity: 32,
            resource_group: "person-ai-document-parse".into(),
            resource_group_capacity: 32,
            weight: 1,
            supported_tiers: vec![
                "standard".into(),
                "advanced".into(),
                "flash".into(),
                "basic".into(),
            ],
        }])
    }

    async fn capabilities(&self) -> Result<ParsingCapabilities, KnowledgeBaseError> {
        Ok(ParsingCapabilities {
            default_tier: ParsingTier::Standard,
            default_ocr_mode: OcrMode::Text,
            available_tiers: vec![ParsingTier::Standard, ParsingTier::Advanced],
            parser_version: PARSER_VERSION.into(),
            office_tier: ParsingTier::Flash,
        })
    }

    async fn parse(
        &self,
        _filename: &str,
        _bytes: &[u8],
        _tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        Err(KnowledgeBaseError::Invalid("解析需要用户身份".into()))
    }

    async fn parse_for_user(
        &self,
        user: &str,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        self.run(user, filename, bytes, tier, None).await
    }

    async fn parse_durable(
        &self,
        filename: &str,
        bytes: &[u8],
        tier: Option<ParsingTier>,
        checkpoint: Arc<dyn ParseCheckpoint>,
    ) -> Result<StructuredParse, KnowledgeBaseError> {
        let user = checkpoint.task()?.user_code;
        self.run(&user, filename, bytes, tier, Some(checkpoint))
            .await
    }

    async fn cancel_task(
        &self,
        task: &IngestionTask,
    ) -> Result<Option<String>, KnowledgeBaseError> {
        let Some(id) = task.remote_job_id.as_deref() else {
            return Ok(None);
        };
        let (credentials, base_url) = self
            .providers
            .document_parse_session(&task.user_code)
            .await
            .map_err(platform_error)?;
        let context = parse_context(credentials);
        let client = DocumentParseClient::new(base_url).map_err(platform_error)?;
        let current = match client.get_task(&context, id).await {
            Ok(task) => task,
            Err(error) if lifecycle::task_not_found(&error) => return Ok(Some(id.into())),
            Err(error) => return Err(platform_error(error)),
        };
        if current.deleted
            || current.deletion_pending
            || matches!(
                current.status.as_str(),
                "completed" | "failed" | "partial" | "canceled"
            )
        {
            return Ok(Some(id.into()));
        }
        match client.cancel_task(&context, id).await {
            Ok(_) => Ok(Some(id.into())),
            Err(error) if lifecycle::task_not_found(&error) => Ok(Some(id.into())),
            Err(error) => Err(platform_error(error)),
        }
    }
}
