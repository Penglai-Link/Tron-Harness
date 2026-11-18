/// One resolved delivered file, already re-authorized against the stored run
/// root and checked against the delivery fingerprint.
pub struct ArtifactContent {
    pub path: std::path::PathBuf,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: u64,
}

impl ChatService {
    /// Resolve a delivered item by opaque coordinates. The request never
    /// carries a path; the stored record is the only source of truth.
    pub fn artifact_content(
        &self,
        session_code: &str,
        run_code: &str,
        item_code: &str,
    ) -> Result<ArtifactContent, ChatError> {
        self.repository().get_session(session_code)?;
        let mut by_run = self.repository().artifacts_by_session(session_code)?;
        let records = by_run.remove(run_code).ok_or(ChatError::ArtifactNotFound)?;
        let record = records
            .into_iter()
            .find(|record| record.id == item_code)
            .ok_or(ChatError::ArtifactNotFound)?;
        let root = std::path::Path::new(&record.source.root);
        let canonical_root = root
            .canonicalize()
            .map_err(|_| ChatError::ArtifactUnavailable)?;
        let resolved = root
            .join(&record.source.relative_path)
            .canonicalize()
            .map_err(|_| ChatError::ArtifactNotFound)?;
        if !resolved.starts_with(&canonical_root) {
            return Err(ChatError::ArtifactUnavailable);
        }
        let metadata = std::fs::metadata(&resolved).map_err(|_| ChatError::ArtifactNotFound)?;
        if !metadata.is_file() {
            return Err(ChatError::ArtifactNotFound);
        }
        let modified_at_ns = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_nanos().min(u64::MAX as u128) as u64)
            .unwrap_or(0);
        if metadata.len() != record.fingerprint.size
            || modified_at_ns != record.fingerprint.modified_at_ns
        {
            return Err(ChatError::ArtifactChanged);
        }
        Ok(ArtifactContent {
            path: resolved,
            file_name: record.display,
            media_type: record
                .media_type
                .unwrap_or_else(|| "application/octet-stream".into()),
            byte_size: metadata.len(),
        })
    }
}