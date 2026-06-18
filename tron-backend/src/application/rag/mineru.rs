use std::{collections::BTreeMap, time::Duration};

use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::{config::MineruConfig, domain::rag::RagError};

use super::{RagResolvedDocument, document_source::ProjectDocFile};

#[derive(Clone)]
pub(crate) struct MineruDocumentParser {
    client: reqwest::Client,
    config: MineruConfig,
}

impl MineruDocumentParser {
    pub(crate) fn new(client: reqwest::Client, config: MineruConfig) -> Self {
        Self { client, config }
    }

    pub(crate) async fn parse(
        &self,
        file: ProjectDocFile,
    ) -> Result<RagResolvedDocument, RagError> {
        if !self.config.enabled {
            return Err(RagError::DocumentParseFailed);
        }
        let request = self.build_request(file.clone())?;
        let timeout = Duration::from_secs(self.config.timeout_seconds.max(1));
        let response = tokio::time::timeout(timeout, request.send())
            .await
            .map_err(|_| RagError::DocumentParseFailed)?
            .map_err(|_| RagError::DocumentParseFailed)?;
        if !response.status().is_success() {
            return Err(RagError::DocumentParseFailed);
        }
        let payload = response
            .json::<MineruParseResponse>()
            .await
            .map_err(|_| RagError::DocumentParseFailed)?;
        let md_content = payload
            .result_for(&file)
            .and_then(|result| result.md_content.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(RagError::DocumentParseFailed)?
            .to_string();

        Ok(RagResolvedDocument {
            document_uri: file.document_uri,
            content_length: md_content.len(),
            content_text: md_content,
        })
    }

    fn build_request(&self, file: ProjectDocFile) -> Result<reqwest::RequestBuilder, RagError> {
        let bytes = std::fs::read(&file.path).map_err(|_| RagError::SourceResolveFailed)?;
        if bytes.len() > self.config.max_bytes {
            return Err(RagError::DocumentTooLarge);
        }
        let file_part = Part::bytes(bytes).file_name(file.file_name);
        let form = Form::new()
            .part("files", file_part)
            .text("backend", self.config.backend.clone())
            .text("parse_method", self.config.parse_method.clone())
            .text("return_md", "true")
            .text(
                "return_content_list",
                self.config.return_content_list.to_string(),
            )
            .text("response_format_zip", "false");

        Ok(self.client.post(parse_url(&self.config)).multipart(form))
    }
}

#[derive(Debug, Deserialize)]
struct MineruParseResponse {
    results: BTreeMap<String, MineruParseResult>,
}

impl MineruParseResponse {
    fn result_for(&self, file: &ProjectDocFile) -> Option<&MineruParseResult> {
        let stem = file
            .file_name
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(file.file_name.as_str());
        self.results
            .get(stem)
            .or_else(|| self.results.get(&file.file_name))
            .or_else(|| {
                (self.results.len() == 1)
                    .then(|| self.results.values().next())
                    .flatten()
            })
    }
}

#[derive(Debug, Deserialize)]
struct MineruParseResult {
    md_content: Option<String>,
}

fn parse_url(config: &MineruConfig) -> String {
    format!(
        "{}/{}",
        config.base_url.trim_end_matches('/'),
        config.parse_endpoint.trim_start_matches('/')
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    #[test]
    fn mineru_response_prefers_stem_result_key() {
        let payload = serde_json::from_value::<MineruParseResponse>(json!({
            "results": {
                "demo": {
                    "md_content": "# Demo"
                }
            }
        }))
        .expect("payload should deserialize");
        let file = test_file("demo.pdf");

        assert_eq!(
            payload
                .result_for(&file)
                .and_then(|result| result.md_content.as_deref()),
            Some("# Demo")
        );
    }

    #[test]
    fn mineru_response_falls_back_to_single_result() {
        let payload = serde_json::from_value::<MineruParseResponse>(json!({
            "results": {
                "sanitized_demo": {
                    "md_content": "# Demo"
                }
            }
        }))
        .expect("payload should deserialize");
        let file = test_file("demo.pdf");

        assert_eq!(
            payload
                .result_for(&file)
                .and_then(|result| result.md_content.as_deref()),
            Some("# Demo")
        );
    }

    fn test_file(file_name: &str) -> ProjectDocFile {
        ProjectDocFile {
            document_uri: "project-doc://TRON_CORE/docs/demo.pdf".into(),
            path: PathBuf::from(file_name),
            file_name: file_name.into(),
            content_length: 10,
        }
    }
}
