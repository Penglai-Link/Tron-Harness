use std::{future::Future, path::PathBuf, pin::Pin};

use crate::domain::rag::RagError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagDocumentSourceCall {
    pub workspace_code: String,
    pub user_code: String,
    pub agent_code: String,
    pub document_uri: String,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagDocumentParseCall {
    pub workspace_code: String,
    pub user_code: String,
    pub agent_code: String,
    pub document_uri: String,
    pub document_format: String,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagResolvedDocument {
    pub document_uri: String,
    pub content_text: String,
    pub content_length: usize,
}

pub trait RagDocumentSourceRuntime: Clone + Send + Sync + 'static {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>>;

    fn resolve_project_doc_file<'a>(
        &'a self,
        call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async move { self.resolve_project_doc_markdown(call).await })
    }

    fn parse_project_doc_with_mineru<'a>(
        &'a self,
        _call: RagDocumentParseCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async { Err(RagError::DocumentParseFailed) })
    }
}

pub(crate) fn resolve_project_doc_markdown_file(
    project_doc_root: PathBuf,
    call: &RagDocumentSourceCall,
) -> Result<RagResolvedDocument, RagError> {
    let resolved = resolve_project_doc_file_path(project_doc_root, call)?;
    let content =
        std::fs::read_to_string(&resolved.path).map_err(|_| RagError::DocumentParseFailed)?;
    if content.as_bytes().len() > call.max_bytes {
        return Err(RagError::DocumentTooLarge);
    }
    Ok(RagResolvedDocument {
        document_uri: call.document_uri.clone(),
        content_length: content.len(),
        content_text: content,
    })
}

pub(crate) fn resolve_project_doc_reference_file(
    project_doc_root: PathBuf,
    call: &RagDocumentSourceCall,
) -> Result<RagResolvedDocument, RagError> {
    let resolved = resolve_project_doc_file_path(project_doc_root, call)?;
    Ok(RagResolvedDocument {
        document_uri: call.document_uri.clone(),
        content_length: resolved.content_length,
        content_text: String::new(),
    })
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectDocFile {
    pub document_uri: String,
    pub path: PathBuf,
    pub file_name: String,
    pub content_length: usize,
}

pub(crate) fn resolve_project_doc_parse_file(
    project_doc_root: PathBuf,
    call: &RagDocumentParseCall,
) -> Result<ProjectDocFile, RagError> {
    resolve_project_doc_file_path(
        project_doc_root,
        &RagDocumentSourceCall {
            workspace_code: call.workspace_code.clone(),
            user_code: call.user_code.clone(),
            agent_code: call.agent_code.clone(),
            document_uri: call.document_uri.clone(),
            max_bytes: call.max_bytes,
        },
    )
}

fn resolve_project_doc_file_path(
    project_doc_root: PathBuf,
    call: &RagDocumentSourceCall,
) -> Result<ProjectDocFile, RagError> {
    let relative_path = project_doc_relative_path(&call.document_uri, &call.workspace_code)?;
    let root = canonical_root(project_doc_root)?;
    let path = root.join(relative_path);
    let canonical_path = path
        .canonicalize()
        .map_err(|_| RagError::SourceResolveFailed)?;
    if !canonical_path.starts_with(&root) {
        return Err(RagError::SourceResolveFailed);
    }

    let metadata = std::fs::metadata(&canonical_path).map_err(|_| RagError::SourceResolveFailed)?;
    if !metadata.is_file() {
        return Err(RagError::SourceResolveFailed);
    }
    let content_length = usize::try_from(metadata.len()).map_err(|_| RagError::DocumentTooLarge)?;
    if content_length > call.max_bytes {
        return Err(RagError::DocumentTooLarge);
    }
    let file_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RagError::SourceResolveFailed)?
        .to_string();
    Ok(ProjectDocFile {
        document_uri: call.document_uri.clone(),
        path: canonical_path,
        file_name,
        content_length,
    })
}

fn canonical_root(project_doc_root: PathBuf) -> Result<PathBuf, RagError> {
    let root = if project_doc_root.is_absolute() {
        project_doc_root
    } else {
        std::env::current_dir()
            .map_err(|_| RagError::SourceResolveFailed)?
            .join(project_doc_root)
    };
    root.canonicalize()
        .map_err(|_| RagError::SourceResolveFailed)
}

fn project_doc_relative_path(
    document_uri: &str,
    workspace_code: &str,
) -> Result<PathBuf, RagError> {
    let rest = document_uri
        .strip_prefix("project-doc://")
        .ok_or(RagError::SourceResolveFailed)?;
    let (uri_workspace, raw_path) = rest.split_once('/').ok_or(RagError::SourceResolveFailed)?;
    if uri_workspace != workspace_code || raw_path.trim().is_empty() {
        return Err(RagError::SourceResolveFailed);
    }

    let mut path = PathBuf::new();
    for segment in raw_path.split('/') {
        if segment.is_empty() || matches!(segment, "." | "..") {
            return Err(RagError::SourceResolveFailed);
        }
        path.push(segment);
    }
    Ok(path)
}
