use std::io::Read;

use rand::RngCore;

use crate::{
    application::resource::ChatSessionAttachmentCreateCommand,
    domain::resource::{
        ChatSessionAttachmentRecord, DEFAULT_BINARY_STORAGE_PROFILE, ResourceError,
        validate_binary_name, validate_resource_code,
    },
    shared::time::current_timestamp,
};

use super::{ResourceService, binaries::spool_limited_binary};

/// Chat files are intentionally smaller than general resources. They are sent
/// to a model and are private to one session, so allowing the resource module's
/// normal multi-gigabyte limit here would be unsafe and surprising.
pub const MAX_CHAT_SESSION_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const MAX_CHAT_SESSION_ATTACHMENT_CONTENT_TYPE_CHARS: usize = 255;

impl ResourceService {
    pub fn max_chat_session_attachment_bytes(&self) -> u64 {
        self.config
            .limits
            .max_binary_bytes
            .min(MAX_CHAT_SESSION_ATTACHMENT_BYTES)
    }

    pub fn store_chat_session_attachment(
        &self,
        command: ChatSessionAttachmentCreateCommand,
        source: &mut dyn Read,
    ) -> Result<ChatSessionAttachmentRecord, ResourceError> {
        validate_resource_code(&command.workspace_code)?;
        validate_resource_code(&command.session_code)?;
        validate_resource_code(&command.attachment_code)?;
        validate_binary_name(
            &command.file_name,
            self.config.limits.max_binary_name_length,
        )?;
        let content_type = normalize_content_type(&command.content_type)?;
        let (temporary, byte_size) =
            spool_limited_binary(source, self.max_chat_session_attachment_bytes())?;
        if byte_size == 0 {
            return Err(ResourceError::InvalidInput);
        }
        let object_key = self.next_chat_session_attachment_object_key(
            &command.workspace_code,
            &command.session_code,
            &command.attachment_code,
        );
        let (storage_locator, stored_byte_size) = self.storage.put_file(
            DEFAULT_BINARY_STORAGE_PROFILE,
            &object_key,
            temporary.path(),
        )?;
        if stored_byte_size != byte_size {
            let _ = self.storage.delete(&storage_locator);
            return Err(ResourceError::StorageFailure);
        }
        let record = ChatSessionAttachmentRecord {
            workspace_code: command.workspace_code,
            session_code: command.session_code,
            attachment_code: command.attachment_code,
            file_name: command.file_name,
            content_type,
            byte_size,
            storage_locator,
            created_at: current_timestamp(),
        };
        if let Err(error) =
            self.repository(|repository| repository.insert_chat_session_attachment(&record))
        {
            self.delete_unreferenced_locator(&record.storage_locator);
            return Err(error);
        }
        Ok(record)
    }

    pub fn read_chat_session_attachment(
        &self,
        workspace_code: &str,
        session_code: &str,
        attachment_code: &str,
    ) -> Result<(ChatSessionAttachmentRecord, Vec<u8>), ResourceError> {
        let record = self.repository(|repository| {
            repository.get_chat_session_attachment(workspace_code, session_code, attachment_code)
        })?;
        let maximum = self.max_chat_session_attachment_bytes();
        if record.byte_size == 0 || record.byte_size > maximum {
            return Err(ResourceError::MissingBinaryObject);
        }
        let mut bytes = Vec::with_capacity(record.byte_size as usize);
        let copied = self
            .storage
            .get_writer(&record.storage_locator, &mut bytes)?;
        if copied != record.byte_size || bytes.len() as u64 != record.byte_size {
            return Err(ResourceError::StorageFailure);
        }
        Ok((record, bytes))
    }

    pub fn read_chat_session_attachment_range(
        &self,
        workspace_code: &str,
        session_code: &str,
        attachment_code: &str,
        start_byte: u64,
        max_bytes: u64,
    ) -> Result<(ChatSessionAttachmentRecord, u64, Vec<u8>), ResourceError> {
        let record = self.repository(|repository| {
            repository.get_chat_session_attachment(workspace_code, session_code, attachment_code)
        })?;
        let maximum = self.max_chat_session_attachment_bytes();
        if record.byte_size == 0 || record.byte_size > maximum {
            return Err(ResourceError::MissingBinaryObject);
        }
        if start_byte > record.byte_size {
            return Err(ResourceError::InvalidInput);
        }
        let length = max_bytes.min(record.byte_size.saturating_sub(start_byte));
        let mut bytes = Vec::with_capacity(length as usize);
        let copied = self.storage.get_range_writer(
            &record.storage_locator,
            start_byte,
            length,
            &mut bytes,
        )?;
        if copied != length || bytes.len() as u64 != length {
            return Err(ResourceError::StorageFailure);
        }
        Ok((record, start_byte, bytes))
    }

    pub fn chat_session_attachment_records(
        &self,
        session_code: &str,
    ) -> Result<Vec<ChatSessionAttachmentRecord>, ResourceError> {
        self.repository(|repository| repository.list_chat_session_attachments(session_code))
    }

    pub fn cleanup_chat_session_attachment_files(&self, records: &[ChatSessionAttachmentRecord]) {
        for record in records {
            if let Err(error) = self.storage.delete(&record.storage_locator) {
                eprintln!(
                    "attachment {} file cleanup failed after session deletion: {error:?}",
                    record.attachment_code
                );
            }
        }
    }

    fn next_chat_session_attachment_object_key(
        &self,
        workspace_code: &str,
        session_code: &str,
        attachment_code: &str,
    ) -> String {
        format!(
            "chat-sessions/{workspace_code}/{session_code}/{attachment_code}/{:016X}",
            rand::rng().next_u64(),
        )
    }
}

fn normalize_content_type(value: &str) -> Result<String, ResourceError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok("application/octet-stream".to_string());
    }
    if value.chars().count() > MAX_CHAT_SESSION_ATTACHMENT_CONTENT_TYPE_CHARS
        || value.chars().any(char::is_control)
    {
        return Err(ResourceError::InvalidInput);
    }
    Ok(value.to_string())
}
