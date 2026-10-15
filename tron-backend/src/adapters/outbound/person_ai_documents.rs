//! Person-AI document parsing transport. The platform owns submission, billing,
//! and result files; Tron owns the reusable parse artifact and RAG index.
use crate::application::{
    knowledge_base::{
        KnowledgeBaseError,
        ingestion::{IngestionTask, ParseEndpointCapacity},
        local_documents::{
            DocumentLocation, ImageAnalysisSummary, ImageAnnotation, LocatedSection,
            MAX_EXPANDED_BYTES, MAX_TEXT_BYTES, parse_local_document,
        },
        parsing::{
            KnowledgeDocumentParser, OcrMode, ParseCheckpoint, ParsedAsset, ParsingCapabilities,
            ParsingTier, StructuredParse,
        },
    },
    model_provider::ModelProviderService,
};
use async_trait::async_trait;
use crab_serverapi_parse::{
    base::AuthorizedClientApi,
    document_parse::{CreateParseTask, DocumentParseClient, ParseMode, ParseTask},
};
use reqwest::Method;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};

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
        if self.ocr_mode == OcrMode::Ocr {
            ParseMode::Ocr
        } else if tier == Some(ParsingTier::Advanced) {
            ParseMode::Precise
        } else {
            ParseMode::Text
        }
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
        let request_id = prior.as_ref().map(|task| task.id.clone());
        let (credentials, base_url) =
            self.providers
                .document_parse_session(user)
                .await
                .map_err(|error| {
                    KnowledgeBaseError::Storage(format!("Person-AI 解析账号暂不可用：{error}"))
                })?;
        let context = parse_context(credentials);
        let client = DocumentParseClient::new(base_url).map_err(platform_error)?;
        let task = if let Some(id) = prior
            .as_ref()
            .and_then(|task| task.remote_job_id.as_deref())
        {
            client
                .get_task(&context, id)
                .await
                .map_err(platform_error)?
        } else {
            let mut request = CreateParseTask::new(filename, bytes.to_vec(), mode);
            if let Some(id) = request_id {
                request.request_id = id;
            }
            // This checkpoint is written before POST. A lost response is retried
            // with the same request_id, file bytes and mode.
            if let Some(checkpoint) = &checkpoint {
                checkpoint.save(Some(request.request_id.clone()), None, true)?;
            }
            let created = client
                .create_task(&context, &request)
                .await
                .map_err(platform_error)?;
            if let Some(checkpoint) = &checkpoint {
                checkpoint.save(Some(request.request_id), Some(created.id.clone()), false)?;
            }
            created
        };
        let task = self
            .wait_for_settlement(user, task, checkpoint.as_ref())
            .await?;
        self.result(user, filename, bytes, tier, task).await
    }

    async fn wait_for_settlement(
        &self,
        user: &str,
        mut task: ParseTask,
        checkpoint: Option<&Arc<dyn ParseCheckpoint>>,
    ) -> Result<ParseTask, KnowledgeBaseError> {
        loop {
            if task.status == "submission_unknown" {
                return Err(KnowledgeBaseError::Invalid(
                    "平台未确认解析提交；请核查原任务，不能自动重新提交".into(),
                ));
            }
            if task.billing_status == "balance_insufficient" {
                return Err(KnowledgeBaseError::Storage(
                    "解析结果待结算：积分不足，请充值后重试原任务".into(),
                ));
            }
            if task.is_settled() {
                if task.status != "completed" {
                    return Err(KnowledgeBaseError::Invalid(format!(
                        "解析任务{}，未发布不完整的索引：{}",
                        task.status,
                        task.error_message.as_deref().unwrap_or("请检查解析结果")
                    )));
                }
                return Ok(task);
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
            task = client
                .get_task(&context, &task.id)
                .await
                .map_err(platform_error)?;
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
        let result = task.result.as_ref().ok_or_else(|| {
            KnowledgeBaseError::Storage("平台已结算，但未返回解析产物清单".into())
        })?;
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
        let markdown = download(&client, &context, &task.id, markdown_id, MAX_TEXT_BYTES).await?;
        let middle = download(&client, &context, &task.id, middle_id, MAX_EXPANDED_BYTES).await?;
        let assets = if let Ok(zip_id) = output_id(file, "zip") {
            let archive = download_to_temp(&client, &context, &task.id, zip_id).await?;
            images_from_result_zip(&archive)?
        } else {
            Vec::new()
        };
        let middle: Value = serde_json::from_slice(&middle)
            .map_err(|_| KnowledgeBaseError::Storage("平台解析结构化结果无效".into()))?;
        let markdown_text = String::from_utf8(markdown)
            .map_err(|_| KnowledgeBaseError::Storage("平台 Markdown 结果不是 UTF-8".into()))?;
        let (image_text, skipped_images) = image_text_for_index(&markdown_text);
        let index_text = table_text_for_index(&image_text);
        let mut document = parse_local_document("platform-result.md", index_text.as_bytes())
            .map_err(|error| KnowledgeBaseError::Invalid(error.to_string()))?;
        if skipped_images > 0 {
            document.warnings.push(format!(
                "{skipped_images} 张图片没有可用描述或 OCR 文字，已保留解析产物但未建立图像语义索引"
            ));
        }
        if !assets.is_empty() {
            let descriptions = image_references(&markdown_text);
            let mut source_annotations = 0;
            for asset in &assets {
                let alt = descriptions.get(&asset.path).cloned().unwrap_or_default();
                let annotations = if alt.is_empty() {
                    Vec::new()
                } else {
                    source_annotations += 1;
                    vec![ImageAnnotation {
                        kind: "source_caption".into(),
                        source: "person-ai.markdown.alt".into(),
                        text: alt,
                        page: None,
                        block: asset.path.clone(),
                        model_code: None,
                        artifact_resource_code: None,
                    }]
                };
                document.sections.push(LocatedSection {
                    text: String::new(),
                    heading_level: None,
                    location: DocumentLocation {
                        image_resource_codes: vec![asset.path.clone()],
                        image_annotations: annotations,
                        block_kind: Some("image".into()),
                        blocks: vec![asset.path.clone()],
                        ..Default::default()
                    },
                });
            }
            document.image_analysis = Some(ImageAnalysisSummary {
                image_count: assets.len(),
                originals_only: assets.len() - source_annotations,
                with_source_annotations: source_annotations,
                with_generated_descriptions: 0,
                with_unattributed_body_text: 0,
                generation_mode: "parser_controlled".into(),
                supplement_status: None,
            });
        }
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
    context: &crab_serverapi_parse::AuthorizedClientContext,
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

fn image_references(markdown: &str) -> BTreeMap<String, String> {
    let mut references = BTreeMap::new();
    let mut remaining = markdown;
    while let Some(start) = remaining.find("![") {
        let image = &remaining[start + 2..];
        let Some(end_alt) = image.find("](") else {
            break;
        };
        let Some(end_link) = image[end_alt + 2..].find(')') else {
            break;
        };
        let path = image[end_alt + 2..end_alt + 2 + end_link].trim();
        let alt = image[..end_alt].trim();
        if !alt.is_empty()
            && !matches!(
                alt.to_ascii_lowercase().as_str(),
                "image" | "图片" | "figure"
            )
        {
            references.insert(path.to_string(), alt.to_string());
        }
        remaining = &image[end_alt + 2 + end_link + 1..];
    }
    references
}

/// Embedding receives only words from image references. The raw Markdown is
/// retained separately for provenance and display.
fn image_text_for_index(markdown: &str) -> (String, usize) {
    let mut output = String::with_capacity(markdown.len());
    let mut remaining = markdown;
    let mut skipped = 0;
    while let Some(start) = remaining.find("![") {
        output.push_str(&remaining[..start]);
        let image = &remaining[start + 2..];
        let Some(end_alt) = image.find("](") else {
            output.push_str(&remaining[start..]);
            return (output, skipped);
        };
        let Some(end_link) = image[end_alt + 2..].find(')') else {
            output.push_str(&remaining[start..]);
            return (output, skipped);
        };
        let alt = image[..end_alt].trim();
        let looks_like_file = alt.contains('/')
            || alt.contains('\\')
            || [".png", ".jpg", ".jpeg", ".webp", ".gif", ".svg"]
                .iter()
                .any(|suffix| alt.to_ascii_lowercase().ends_with(suffix));
        if !alt.is_empty()
            && !looks_like_file
            && !matches!(
                alt.to_ascii_lowercase().as_str(),
                "image" | "图片" | "figure"
            )
        {
            output.push_str("图片描述：");
            output.push_str(alt);
        } else {
            skipped += 1;
        }
        remaining = &image[end_alt + 2 + end_link + 1..];
    }
    output.push_str(remaining);
    (output, skipped)
}

/// Render Markdown table rows with their column names so a text embedding
/// retains row/column relationships when the chunker splits a large table.
fn table_text_for_index(markdown: &str) -> String {
    fn cells(line: &str) -> Vec<&str> {
        line.trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect()
    }
    fn separator(line: &str, width: usize) -> bool {
        let parts = cells(line);
        parts.len() == width
            && parts.iter().all(|part| {
                let part = part.trim_matches(':');
                part.len() >= 3 && part.bytes().all(|byte| byte == b'-')
            })
    }
    let lines = markdown.lines().collect::<Vec<_>>();
    let mut result = String::with_capacity(markdown.len());
    let mut index = 0;
    let mut fence = false;
    while index < lines.len() {
        let line = lines[index];
        if line.trim_start().starts_with("```") {
            fence = !fence;
        }
        let headers = cells(line);
        if !fence
            && line.contains('|')
            && headers.len() > 1
            && index + 1 < lines.len()
            && separator(lines[index + 1], headers.len())
        {
            result.push_str("表格列：");
            result.push_str(&headers.join("；"));
            result.push('\n');
            result.push('\n');
            index += 2;
            let mut row = 1;
            while index < lines.len() && lines[index].contains('|') {
                let values = cells(lines[index]);
                if values.len() != headers.len() {
                    break;
                }
                result.push_str(&format!("表格第{row}行："));
                for (position, (header, value)) in headers.iter().zip(values).enumerate() {
                    if position > 0 {
                        result.push_str("；");
                    }
                    result.push_str(header);
                    result.push('：');
                    result.push_str(value);
                }
                result.push('\n');
                if result.len() > MAX_TEXT_BYTES {
                    return markdown.to_owned();
                }
                row += 1;
                index += 1;
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
        if result.len() > MAX_TEXT_BYTES {
            return markdown.to_owned();
        }
        index += 1;
    }
    result
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
    context: &crab_serverapi_parse::AuthorizedClientContext,
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
) -> crab_serverapi_parse::AuthorizedClientContext {
    crab_serverapi_parse::AuthorizedClientContext {
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
        let client = DocumentParseClient::new(base_url.clone()).map_err(platform_error)?;
        let current = client
            .get_task(&context, id)
            .await
            .map_err(platform_error)?;
        if matches!(
            current.status.as_str(),
            "completed" | "failed" | "partial" | "canceled"
        ) {
            return Ok(Some(id.into()));
        }
        let api = AuthorizedClientApi::new(base_url).map_err(platform_error)?;
        let request = api
            .new_authorized_request(Method::POST, "/document-parsing/cancel-task", &context)
            .map_err(platform_error)?
            .json(&json!({"id":id}));
        let _: ParseTask = api
            .execute_for_object(request, Some(&context), "取消文件解析任务")
            .await
            .map_err(platform_error)?;
        Ok(Some(id.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{ZipWriter, write::SimpleFileOptions};

    #[test]
    fn selects_only_platform_result_ids_from_the_requested_file() {
        let file = json!({"outputs":{"markdown":"/result/merged.md","middle_json":{"file_id":"middle-1"}}});
        assert_eq!(output_id(&file, "markdown").unwrap(), "/result/merged.md");
        assert_eq!(output_id(&file, "middle_json").unwrap(), "middle-1");
        assert!(output_id(&file, "zip").is_err());
    }

    #[test]
    fn image_links_never_become_text_embeddings() {
        let (index, skipped) = image_text_for_index(
            "表格 | 数量\n![折线图：收入增长](images/chart.png) ![image](images/raw.png)",
        );
        assert_eq!(index, "表格 | 数量\n图片描述：折线图：收入增长 ");
        assert_eq!(skipped, 1);
    }

    #[test]
    fn table_rows_keep_column_relationships_for_text_embedding() {
        let text = "| 产品 | 数量 |\n| --- | ---: |\n| A | 3 |\n| B | 5 |";
        let index = table_text_for_index(text);
        assert!(index.contains("表格第1行：产品：A；数量：3"));
        assert!(index.contains("表格第2行：产品：B；数量：5"));
    }

    #[test]
    fn extracts_nested_platform_images_without_loading_archives_into_memory() {
        let mut inner = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        inner
            .start_file("images/chart.png", SimpleFileOptions::default())
            .unwrap();
        inner.write_all(b"image-bytes").unwrap();
        let nested = inner.finish().unwrap().into_inner();
        let archive = tempfile::NamedTempFile::new().unwrap();
        let mut outer = ZipWriter::new(archive.reopen().unwrap());
        outer
            .start_file("zip/document_0.zip", SimpleFileOptions::default())
            .unwrap();
        outer.write_all(&nested).unwrap();
        outer.finish().unwrap();
        let images = images_from_result_zip(&archive).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].path, "images/chart.png");
        assert_eq!(images[0].bytes, b"image-bytes");
    }
}
