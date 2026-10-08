use super::{
    KnowledgeBase, KnowledgeBaseDocument, KnowledgeBaseError, KnowledgeBaseService,
    ResourceBinaryDownloadCommand, ResourceCommitBinary, ResourceCreateCommand, ResourceKind, hash,
    invalid_storage, new_id, parse_local_document,
};
use super::{local_documents::ParsedDocument, parsing::ParsingArtifact};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, io::Write};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredParse {
    document: ParsedDocument,
    middle: Value,
    structured: Value,
    markdown: String,
    image_resources: BTreeMap<String, String>,
    #[serde(default)]
    rendered_sections: Vec<super::local_documents::LocatedSection>,
}
impl KnowledgeBaseService {
    pub(super) fn derived_resource(
        &self,
        user: &str,
        workspace: &str,
        document: &KnowledgeBaseDocument,
        name: &str,
        kind: ResourceKind,
        bytes: &[u8],
    ) -> Result<String, KnowledgeBaseError> {
        let _guard = self.cleanup_gate.lock().map_err(invalid_storage)?;
        let code = format!("KB_{}", new_id());
        self.repository.register_derived_resource(document, workspace, &code)?;
        // Register privacy before publication. Unlinked failed-generation assets remain private.
        self.repository.mark_private(user, workspace, &code)?;
        let mut temporary = tempfile::NamedTempFile::new().map_err(invalid_storage)?;
        temporary.write_all(bytes).map_err(invalid_storage)?;
        self.resources.create_file_resource(
            user,
            ResourceCreateCommand {
                workspace_code: workspace.into(),
                resource_code: code.clone(),
                kind,
                name: name.into(),
                description: "知识库解析附件".into(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temporary.reopen().map_err(invalid_storage)?,
                file_name: name.into(),
            }),
        )?;
        Ok(code)
    }
    pub(super) async fn parsed_document(
        &self,
        user: &str,
        workspace: &str,
        kb: &KnowledgeBase,
        doc: &KnowledgeBaseDocument,
        bytes: &[u8],
    ) -> Result<(ParsedDocument, KnowledgeBaseDocument), KnowledgeBaseError> {
        if super::fidelity::blocked(doc) { return Err(KnowledgeBaseError::Invalid("数据保真核验发现未解决字形或符号，禁止发布索引；请核对原件及图片".into())); }
        if doc.file_name.to_ascii_lowercase().ends_with(".md") {
            let mut parsed = parse_local_document(&doc.file_name, bytes)
                .map_err(|e| KnowledgeBaseError::Invalid(e.to_string()))?;
            // Keep Markdown references intact; remote or filesystem links are never fetched.
            if std::str::from_utf8(bytes).unwrap_or("").contains("![") {
                parsed.warnings.push(
                    "Markdown图片链接原样保留；仅已有资源引用可供授权Agent读取，不自动下载外部图片"
                        .into(),
                );
            }
            return Ok((parsed, doc.clone()));
        }
        let parser = self.parser.as_ref().ok_or_else(|| {
            KnowledgeBaseError::Invalid("文档解析服务未配置；不会自动降级为纯文本解析".into())
        })?;
        let snapshot = parser.snapshot();
        let parser = snapshot.as_ref().unwrap_or(parser);
        let office = doc.file_name.to_ascii_lowercase().ends_with(".docx")
            || doc.file_name.to_ascii_lowercase().ends_with(".xlsx");
        let tier = if office {
            super::ParsingTier::Flash
        } else {
            doc.parsing_tier.unwrap_or(parser.default_tier())
        };
        // Preflight native charts before the local parser/renderer handles any active content.
        let chart_plan = if office {
            let mut zip =
                zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(invalid_storage)?;
            let mut charts = false;
            for i in 0..zip.len() {
                let file = zip.by_index(i).map_err(invalid_storage)?;
                if file.name().contains("/charts/") && file.name().ends_with(".xml") {
                    charts = true;
                    break;
                }
            }
            if charts {
                Some(super::office_visuals::validate_office_chart_input(
                    &doc.file_name,
                    bytes,
                )?)
            } else {
                None
            }
        } else {
            None
        };
        let schema = if chart_plan.as_ref().is_some_and(|p| p.has_charts()) {
            "mineru-v1-middle-v2:office-chart-v1"
        } else {
            "mineru-v1-middle-v2"
        };
        let identity = parser.profile_identity();
        let parser_version = match identity["expectedParserVersion"].as_str() {
            Some(version) => version.to_owned(),
            None => parser.capabilities().await?.parser_version,
        };
        // Operational snapshots must not invalidate reusable parse output.
        let mut cache_identity = identity.clone();
        if let Some(identity) = cache_identity.as_object_mut() {
            identity.remove("runtimeConfig");
        }
        let signature = hash(&format!(
            "{}:{}:{}:{}:{schema}:{parser_version}:{cache_identity}",
            doc.resource_code,
            doc.resource_revision,
            tier.as_str(),
            hash_bytes(bytes)
        ));
        if let Some(artifact) = doc
            .parsing_artifact
            .as_ref()
            .filter(|a| a.signature == signature && a.parser_version == parser_version)
        {
            let (_, record) = self.resources.download_binary_to_file(
                user,
                ResourceBinaryDownloadCommand {
                    workspace_code: workspace.into(),
                    resource_code: artifact.resource_code.clone(),
                },
            )?;
            let mut data = Vec::new();
            self.resources.download_binary_storage(&record, &mut data)?;
            if data.len() > super::local_documents::MAX_EXPANDED_BYTES {
                return Err(KnowledgeBaseError::Invalid("解析缓存超过上限".into()));
            }
            let parsed: StoredParse = serde_json::from_slice(&data).map_err(invalid_storage)?;
            let paths = parsed.image_resources.keys().cloned().collect();
            let mut document = super::parsing::normalize_middle(&parsed.middle, &paths)?;
            super::source_locations::apply(&mut document, bytes, &parsed.middle)?;
            map_image_resources(&mut document, &parsed.image_resources)?;
            super::office_visuals::append_rendered_sections(
                &mut document,
                parsed.rendered_sections,
            )?;
            return Ok((document,doc.clone()));
        }
        let mut result = if let Some(checkpoint) = &self.parse_checkpoint {
            parser.parse_durable(&doc.file_name, bytes, doc.parsing_tier, checkpoint.clone()).await?
        } else { parser.parse(&doc.file_name, bytes, doc.parsing_tier).await? };
        super::source_locations::apply(&mut result.document, bytes, &result.middle)?;
        let mut rendered_sections = Vec::new();
        let mut rendered_pdf_resource = None;
        if let Some(plan) = chart_plan.filter(|p| p.has_charts()) {
            let renderer = self.office_renderer.as_ref().ok_or_else(|| {
                KnowledgeBaseError::Invalid(
                    "Office含原生图表但本地视觉转换未配置；未发布缺失图表的索引".into(),
                )
            })?;
            let pdf = renderer.render_pdf(&doc.file_name, bytes).await?;
            let pages = tokio::task::spawn_blocking({
                let pdf = pdf.clone();
                move || eliteforge_agent_memory::render_pdf_evidence(&pdf, Default::default())
            })
            .await
            .map_err(invalid_storage)?
            .map_err(invalid_storage)?;
            if plan.format == "xlsx" && pages.len() != plan.sheets.len() {
                return Err(KnowledgeBaseError::Invalid(
                    "Excel转换页与Sheet数量不匹配，图表来源不明确；未发布索引".into(),
                ));
            }
            rendered_pdf_resource = Some(self.derived_resource(
                user,
                workspace,
                doc,
                "office-chart-preview.pdf",
                ResourceKind::File,
                &pdf,
            )?);
            for page in pages {
                let key = if plan.format == "xlsx" {
                    plan.sheets
                        .get(page.page as usize - 1)
                        .ok_or_else(|| KnowledgeBaseError::Invalid("Excel转换Sheet缺失".into()))?
                        .as_str()
                } else {
                    "document"
                };
                let refs = plan.chart_sources.get(key).cloned().unwrap_or_default();
                // Only chart-bearing sheets contribute visual evidence. Word page-to-chart
                // association is deliberately not inferred from paragraph order.
                if refs.is_empty() {
                    continue;
                }
                let path = format!("office-rendered/page-{}.png", page.page);
                let source_note = if plan.format == "docx" {
                    "文件内图表段落引用（未断言每个图表位于此转换页）"
                } else {
                    "原始Sheet图表及数据范围"
                };
                let text = format!(
                    "Office本地图表视觉预览，转换页{}（非原文页码）。{}：{}\n[图片资源: {}]",
                    page.page,
                    source_note,
                    refs.join("; "),
                    path
                );
                rendered_sections.push(super::local_documents::LocatedSection {
                    text,
                    heading_level: None,
                    location: super::local_documents::DocumentLocation {
                        rendered_page: Some(page.page),
                        sheet: (plan.format == "xlsx").then(|| key.to_owned()),
                        chart_sources: refs,
                        blocks: vec![format!("office-rendered:{}", page.page)],
                        image_resource_codes: vec![path.clone()],
                        block_kind: Some("image".into()),
                        ..Default::default()
                    },
                });
                result.assets.push(super::parsing::ParsedAsset {
                    path,
                    bytes: page.png,
                });
            }
        }
        let mut image_resources = BTreeMap::new();
        for asset in &result.assets {
            let name = std::path::Path::new(&asset.path)
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| KnowledgeBaseError::Invalid("解析图片名称无效".into()))?;
            let code =
                self.derived_resource(user, workspace, doc, name, ResourceKind::Image, &asset.bytes)?;
            image_resources.insert(asset.path.clone(), code);
        }
        map_image_resources(&mut result.document, &image_resources)?;
        let mut visuals = super::local_documents::ParsedDocument {
            format: result.document.format.clone(),
            sections: rendered_sections,
            warnings: vec![],
            image_analysis: None,
        };
        map_image_resources(&mut visuals, &image_resources)?;
        let rendered_sections = visuals.sections;
        super::office_visuals::append_rendered_sections(
            &mut result.document,
            rendered_sections.clone(),
        )?;
        // Store the complete original structured representations, including bbox/table/formula
        // and unrecognized vendor blocks, together with their explicit Resource mapping.
        let stored = StoredParse {
            document: result.document.clone(),
            middle: result.middle,
            structured: result.structured,
            markdown: result.markdown,
            image_resources: image_resources.clone(),
            rendered_sections,
        };
        let data = serde_json::to_vec(&stored).map_err(invalid_storage)?;
        if data.len() > super::local_documents::MAX_EXPANDED_BYTES {
            return Err(KnowledgeBaseError::Invalid(
                "解析结构化资料超过100 MiB上限".into(),
            ));
        }
        let resource_code = self.derived_resource(
            user,
            workspace,
            doc,
            "parsed-document.json",
            ResourceKind::File,
            &data,
        )?;
        let mut cached = doc.clone();
        cached.parsing_artifact = Some(ParsingArtifact {
            signature,
            resource_code,
            effective_tier: result.effective_tier,
            parser_version: result.parser_version,
            vendor_job_id: result.vendor_job_id,
            image_resource_codes: image_resources.into_values().collect(),
            fidelity_checked: false,
            fidelity_resolution: None,
            fidelity_issues: vec![],
        });
        if let Some(artifact) = &cached.parsing_artifact {
            cached
                .derived_resource_codes
                .push(artifact.resource_code.clone());
            cached
                .derived_resource_codes
                .extend(artifact.image_resource_codes.iter().cloned());
            cached.derived_resource_codes.sort();
            cached.derived_resource_codes.dedup();
        }
        if let Some(code) = rendered_pdf_resource {
            cached.derived_resource_codes.push(code);
            cached.derived_resource_codes.sort();
            cached.derived_resource_codes.dedup();
        }
        self.repository.save_parsing_artifact(&kb.id, &cached)?;
        Ok((result.document, cached))
    }
}
fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn map_image_resources(
    document: &mut ParsedDocument,
    resources: &BTreeMap<String, String>,
) -> Result<(), KnowledgeBaseError> {
    for section in &mut document.sections {
        for path in &mut section.location.image_resource_codes {
            *path = resources
                .get(path)
                .ok_or_else(|| KnowledgeBaseError::Invalid("解析图片引用缺失".into()))?
                .clone();
        }
        for (path, code) in resources {
            section.text = section.text.replace(path, &format!("resource:{code}"));
        }
    }
    Ok(())
}

pub(super) fn cached_document_for_fidelity(bytes:&[u8])->Result<ParsedDocument,KnowledgeBaseError>{
    let cached:StoredParse=serde_json::from_slice(bytes).map_err(invalid_storage)?;Ok(cached.document)
}
