use std::io::Write;

use crate::domain::resource::{
    ChatSessionAttachmentRecord, ResourceError, ResourceFileRecord,
};

pub trait ResourceRepository: Send {
    fn list_files(&mut self) -> Result<Vec<ResourceFileRecord>, ResourceError>;
    fn insert_file(&mut self, record: &ResourceFileRecord) -> Result<(), ResourceError>;
    fn update_file(&mut self, record: &ResourceFileRecord) -> Result<bool, ResourceError>;
    fn delete_file(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
        revision: u64,
    ) -> Result<bool, ResourceError>;

    fn insert_chat_session_attachment(
        &mut self,
        record: &ChatSessionAttachmentRecord,
    ) -> Result<(), ResourceError>;
    fn get_chat_session_attachment(
        &mut self,
        workspace_code: &str,
        session_code: &str,
        attachment_code: &str,
    ) -> Result<ChatSessionAttachmentRecord, ResourceError>;
    fn list_chat_session_attachments(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ChatSessionAttachmentRecord>, ResourceError>;
}

pub trait ObjectStorage: Send + Sync {
    fn put_file(&self, object_key: &str, source: &std::path::Path) -> Result<u64, ResourceError>;
    fn put_reader(
        &self,
        object_key: &str,
        source: &mut dyn std::io::Read,
    ) -> Result<u64, ResourceError>;
    fn get_file(&self, object_key: &str, target: &std::path::Path) -> Result<u64, ResourceError>;
    fn get_writer(
        &self,
        object_key: &str,
        target: &mut dyn std::io::Write,
    ) -> Result<u64, ResourceError>;
    /// Stream a bounded byte range into the target. Backends with native range
    /// support should override this method; the default keeps memory bounded
    /// while remaining compatible with storage implementations that only
    /// expose sequential reads.
    fn get_range_writer(
        &self,
        object_key: &str,
        start_byte: u64,
        max_bytes: u64,
        target: &mut dyn std::io::Write,
    ) -> Result<u64, ResourceError> {
        let mut range = RangeWriter {
            skipped: start_byte,
            remaining: max_bytes,
            written: 0,
            target,
        };
        self.get_writer(object_key, &mut range)?;
        range.flush().map_err(|_| ResourceError::StorageFailure)?;
        Ok(range.written)
    }
    fn delete(&self, object_key: &str) -> Result<(), ResourceError>;
    fn exists(&self, object_key: &str) -> Result<bool, ResourceError>;
}

struct RangeWriter<'a> {
    skipped: u64,
    remaining: u64,
    written: u64,
    target: &'a mut dyn std::io::Write,
}

impl std::io::Write for RangeWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let mut selected = buffer;
        if self.skipped > 0 {
            let skipped = self.skipped.min(buffer.len() as u64) as usize;
            self.skipped -= skipped as u64;
            selected = &buffer[skipped..];
        }
        if self.remaining > 0 && !selected.is_empty() {
            let length = self.remaining.min(selected.len() as u64) as usize;
            self.target.write_all(&selected[..length])?;
            self.remaining -= length as u64;
            self.written += length as u64;
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.target.flush()
    }
}
