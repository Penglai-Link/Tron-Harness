use std::{
    fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use rand::RngCore;

use crate::{
    application::resource::{
        ResourceBinaryAgentCommand, ResourceBinaryDownloadCommand, ResourceBinaryResult,
        ResourceBinaryUploadCommand, ResourceCommitBinary, ResourceCommitResult,
        ResourceCreateCommand, ResourceHttpBinaryDownload,
    },
    domain::resource::{
        DEFAULT_BINARY_STORAGE_PROFILE, ManagedResource, ResourceError, ResourceFileRecord,
        ResourceKind, validate_binary_name, validate_resource_code,
    },
    shared::time::current_timestamp,
};

use super::ResourceService;

impl ResourceService {
    pub fn create_file_resource_from_agent_path(
        &self,
        actor_user_code: &str,
        mut command: ResourceCreateCommand,
        source_path: &str,
        agent_local_directory: Option<&str>,
    ) -> Result<ResourceCommitResult, ResourceError> {
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        let source_path = self.resolve_agent_file_path(source_path, agent_local_directory)?;
        let file_name = source_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.trim().is_empty())
            .ok_or(ResourceError::PathDenied)?;
        let source = fs::File::open(&source_path).map_err(|_| ResourceError::PathDenied)?;
        if source
            .metadata()
            .map_err(|_| ResourceError::PathDenied)?
            .len()
            > self.effective_config().limits.max_binary_bytes
        {
            return Err(ResourceError::InvalidInput);
        }
        command.workspace_code = workspace_code;
        self.create_file_resource(
            actor_user_code,
            command,
            Some(ResourceCommitBinary { source, file_name }),
        )
    }

    pub fn preflight_binary_upload(
        &self,
        actor_user_code: &str,
        command: &ResourceBinaryUploadCommand,
    ) -> Result<(), ResourceError> {
        validate_resource_code(&command.resource_code)?;
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        self.authorize_resource_mutation(actor_user_code, &workspace_code, &command.resource_code)?;
        match self.managed_resource(&workspace_code, &command.resource_code)? {
            Some(ManagedResource::File(_)) => Ok(()),
            Some(_) | None => Err(ResourceError::NotFound),
        }
    }

    pub fn upload_binary_from_reader(
        &self,
        actor_user_code: &str,
        command: ResourceBinaryUploadCommand,
        file_name: &str,
        source: &mut dyn Read,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        let record = match self.managed_resource(&workspace_code, &command.resource_code)? {
            Some(ManagedResource::File(record)) => record,
            Some(_) | None => return Err(ResourceError::NotFound),
        };
        self.authorize_resource_mutation(actor_user_code, &workspace_code, &command.resource_code)?;
        let updated = self.replace_file_contents(actor_user_code, &record, file_name, source)?;
        Ok(ResourceBinaryResult {
            action: "UPLOADED".to_string(),
            resource_code: updated.resource_code,
            file_name: Some(updated.file_name),
            byte_size: Some(updated.byte_size),
            target_path: None,
        })
    }

    pub fn download_binary_to_file(
        &self,
        actor_user_code: &str,
        command: ResourceBinaryDownloadCommand,
    ) -> Result<(ResourceHttpBinaryDownload, ResourceFileRecord), ResourceError> {
        let record = self
            .download_binary_records(
                actor_user_code,
                &command.workspace_code,
                &[command.resource_code],
            )?
            .pop()
            .ok_or(ResourceError::NotFound)?;
        Ok((
            ResourceHttpBinaryDownload {
                file_name: record.file_name.clone(),
                byte_size: record.byte_size,
            },
            record,
        ))
    }

    /// Resolve one authorized metadata snapshot for a batch of binary reads.
    /// Preserve caller order and duplicates; each record retains its own privacy
    /// check and inherited-resource override semantics.
    pub fn download_binary_records(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
        resource_codes: &[String],
    ) -> Result<Vec<ResourceFileRecord>, ResourceError> {
        for code in resource_codes {
            validate_resource_code(code)?;
        }
        let workspace_code = self.workspace_scope(actor_user_code, workspace_code, false)?;
        let effective = self.effective_resources(&workspace_code)?;
        resource_codes
            .iter()
            .map(|code| {
                let record = match effective.get(code).map(|entry| &entry.resource) {
                    Some(ManagedResource::File(record)) => record,
                    Some(_) | None => return Err(ResourceError::NotFound),
                };
                self.authorize_resource_read(
                    actor_user_code,
                    &record.workspace_code,
                    &record.resource_code,
                )?;
                if !self.storage.exists(&record.storage_locator)? {
                    return Err(ResourceError::MissingBinaryObject);
                }
                Ok(record.clone())
            })
            .collect()
    }

    pub fn download_binary_storage(
        &self,
        record: &ResourceFileRecord,
        target: &mut dyn std::io::Write,
    ) -> Result<u64, ResourceError> {
        self.storage.get_writer(&record.storage_locator, target)
    }

    pub fn handle_agent_binary_upload(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
        command: ResourceBinaryAgentCommand,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        self.authorize_workspace_write(actor_user_code, workspace_code)?;
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let source_path = command
            .source_path
            .as_deref()
            .ok_or(ResourceError::InvalidInput)?;
        let source =
            self.resolve_agent_file_path(source_path, command.agent_local_directory.as_deref())?;
        let record = match self.managed_resource(&workspace_code, &command.resource_code)? {
            Some(ManagedResource::File(record)) => record,
            Some(_) | None => return Err(ResourceError::NotFound),
        };
        let file_name = source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.trim().is_empty())
            .ok_or(ResourceError::InvalidInput)?;
        let mut input = fs::File::open(&source).map_err(|_| ResourceError::PathDenied)?;
        if input
            .metadata()
            .map_err(|_| ResourceError::PathDenied)?
            .len()
            > self.effective_config().limits.max_binary_bytes
        {
            return Err(ResourceError::InvalidInput);
        }
        self.authorize_resource_mutation(actor_user_code, &workspace_code, &command.resource_code)?;
        let updated =
            self.replace_file_contents(actor_user_code, &record, &file_name, &mut input)?;
        Ok(ResourceBinaryResult {
            action: "UPLOADED".to_string(),
            resource_code: updated.resource_code,
            file_name: Some(updated.file_name),
            byte_size: Some(updated.byte_size),
            target_path: None,
        })
    }

    pub fn handle_agent_binary_download(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
        command: ResourceBinaryAgentCommand,
    ) -> Result<ResourceBinaryResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        self.authorize_workspace_read(actor_user_code, workspace_code)?;
        let workspace_code = self.active_workspace_code(workspace_code)?;
        let record = match self
            .effective_resources(&workspace_code)?
            .remove(&command.resource_code)
            .map(|effective| effective.resource)
        {
            Some(ManagedResource::File(record)) => record,
            Some(_) | None => return Err(ResourceError::NotFound),
        };
        self.authorize_resource_read(
            actor_user_code,
            &record.workspace_code,
            &record.resource_code,
        )?;
        let target_path = command
            .target_path
            .as_deref()
            .ok_or(ResourceError::InvalidInput)?;
        let target = self.resolve_agent_target_path(
            target_path,
            command.agent_local_directory.as_deref(),
            command.overwrite,
        )?;
        let temporary = sibling_temporary_path(&target)?;
        let copied = match self.storage.get_file(&record.storage_locator, &temporary) {
            Ok(copied) => copied,
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
        };
        if fs::rename(&temporary, &target).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(ResourceError::StorageFailure);
        }
        Ok(ResourceBinaryResult {
            action: "DOWNLOADED".to_string(),
            resource_code: command.resource_code,
            file_name: Some(record.file_name),
            byte_size: Some(copied),
            target_path: Some(target.display().to_string()),
        })
    }

    pub(super) fn store_file_resource(
        &self,
        workspace_code: &str,
        resource_code: &str,
        kind: ResourceKind,
        name: &str,
        description: &str,
        allow_inheritance: bool,
        file_name: &str,
        source: &mut dyn Read,
    ) -> Result<ResourceFileRecord, ResourceError> {
        validate_binary_name(
            file_name,
            self.effective_config().limits.max_binary_name_length,
        )?;
        let (temporary, byte_size) =
            spool_limited_binary(source, self.effective_config().limits.max_binary_bytes)?;
        let object_key = self.next_object_key(workspace_code, resource_code);
        let (locator, stored_byte_size) = self.storage.put_file(
            DEFAULT_BINARY_STORAGE_PROFILE,
            &object_key,
            temporary.path(),
        )?;
        if stored_byte_size != byte_size {
            let _ = self.storage.delete(&locator);
            return Err(ResourceError::StorageFailure);
        }
        let now = current_timestamp();
        let record = ResourceFileRecord {
            workspace_code: workspace_code.to_string(),
            resource_code: resource_code.to_string(),
            kind,
            name: name.trim().to_string(),
            description: description.trim().to_string(),
            file_name: file_name.to_string(),
            file_extension: file_extension(file_name),
            byte_size,
            storage_locator: locator,
            allow_inheritance,
            revision: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        if let Err(error) = self.repository(|repository| repository.insert_file(&record)) {
            self.delete_unreferenced_locator(&record.storage_locator);
            return Err(error);
        }
        Ok(record)
    }

    fn replace_file_contents(
        &self,
        actor_user_code: &str,
        record: &ResourceFileRecord,
        file_name: &str,
        source: &mut dyn Read,
    ) -> Result<ResourceFileRecord, ResourceError> {
        validate_binary_name(
            file_name,
            self.effective_config().limits.max_binary_name_length,
        )?;
        let (temporary, byte_size) =
            spool_limited_binary(source, self.effective_config().limits.max_binary_bytes)?;
        let object_key = self.next_object_key(&record.workspace_code, &record.resource_code);
        let (locator, stored_byte_size) = self.storage.put_file(
            DEFAULT_BINARY_STORAGE_PROFILE,
            &object_key,
            temporary.path(),
        )?;
        if stored_byte_size != byte_size {
            let _ = self.storage.delete(&locator);
            return Err(ResourceError::StorageFailure);
        }
        let previous_locator = record.storage_locator.clone();
        let mut updated = record.clone();
        updated.file_name = file_name.to_string();
        updated.file_extension = file_extension(file_name);
        updated.byte_size = byte_size;
        updated.storage_locator = locator;
        updated.updated_at = current_timestamp();
        match self.repository(|repository| repository.update_file(&updated)) {
            Ok(true) => {}
            Ok(false) => {
                self.delete_unreferenced_locator(&updated.storage_locator);
                return Err(ResourceError::Conflict);
            }
            Err(error) => {
                self.delete_unreferenced_locator(&updated.storage_locator);
                return Err(error);
            }
        }
        self.delete_unreferenced_locator(&previous_locator);
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::File(updated.clone()),
            "resource.updated",
        );
        Ok(updated)
    }

    fn next_object_key(&self, workspace_code: &str, resource_code: &str) -> String {
        format!(
            "resources/{}/{}/{:016X}",
            workspace_code,
            resource_code,
            rand::rng().next_u64(),
        )
    }

    pub(super) fn delete_unreferenced_locator(&self, locator: &str) {
        let _ = self.storage.delete(locator);
    }

    fn resolve_agent_file_path(
        &self,
        raw_path: &str,
        agent_local_directory: Option<&str>,
    ) -> Result<PathBuf, ResourceError> {
        let path = self.resolve_agent_path(raw_path, agent_local_directory, true)?;
        let metadata = fs::symlink_metadata(&path).map_err(|_| ResourceError::PathDenied)?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(ResourceError::PathDenied);
        }
        Ok(path)
    }

    fn resolve_agent_target_path(
        &self,
        raw_path: &str,
        agent_local_directory: Option<&str>,
        overwrite: bool,
    ) -> Result<PathBuf, ResourceError> {
        let path = self.resolve_agent_path(raw_path, agent_local_directory, false)?;
        if path.exists() {
            let metadata = fs::symlink_metadata(&path).map_err(|_| ResourceError::PathDenied)?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() || !overwrite {
                return Err(ResourceError::PathDenied);
            }
        }
        Ok(path)
    }

    fn resolve_agent_path(
        &self,
        raw_path: &str,
        agent_local_directory: Option<&str>,
        must_exist: bool,
    ) -> Result<PathBuf, ResourceError> {
        let raw_path = raw_path.trim();
        if raw_path.is_empty()
            || raw_path.starts_with('~')
            || raw_path.contains('$')
            || raw_path.contains('\0')
        {
            return Err(ResourceError::PathDenied);
        }
        let input = Path::new(raw_path);
        if input
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(ResourceError::PathDenied);
        }
        let temp_root = std::env::temp_dir()
            .canonicalize()
            .map_err(|_| ResourceError::PathDenied)?;
        let local_root = agent_local_directory
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from)
            .map(|path| path.canonicalize().map_err(|_| ResourceError::PathDenied))
            .transpose()?;
        let base = local_root.clone().unwrap_or_else(|| temp_root.clone());
        let candidate = if input.is_absolute() {
            input.to_path_buf()
        } else {
            base.join(input)
        };
        let canonical = if must_exist {
            if fs::symlink_metadata(&candidate)
                .map_err(|_| ResourceError::PathDenied)?
                .file_type()
                .is_symlink()
            {
                return Err(ResourceError::PathDenied);
            }
            candidate
                .canonicalize()
                .map_err(|_| ResourceError::PathDenied)?
        } else {
            let parent = candidate.parent().ok_or(ResourceError::PathDenied)?;
            let parent = parent
                .canonicalize()
                .map_err(|_| ResourceError::PathDenied)?;
            let file_name = candidate.file_name().ok_or(ResourceError::PathDenied)?;
            parent.join(file_name)
        };
        let allowed = canonical.starts_with(&temp_root)
            || local_root
                .as_ref()
                .is_some_and(|root| canonical.starts_with(root));
        if !allowed
            || self
                .storage_roots()
                .iter()
                .any(|root| canonical.starts_with(root))
        {
            return Err(ResourceError::PathDenied);
        }
        Ok(canonical)
    }
}

pub(super) fn spool_limited_binary(
    source: &mut dyn Read,
    maximum_bytes: u64,
) -> Result<(tempfile::NamedTempFile, u64), ResourceError> {
    let mut temporary =
        tempfile::NamedTempFile::new().map_err(|_| ResourceError::StorageFailure)?;
    let sentinel_limit = maximum_bytes.saturating_add(1);
    let mut limited = source.take(sentinel_limit);
    let byte_size = io::copy(&mut limited, temporary.as_file_mut())
        .map_err(|_| ResourceError::StorageFailure)?;
    if byte_size > maximum_bytes {
        return Err(ResourceError::InvalidInput);
    }
    Ok((temporary, byte_size))
}

pub(super) fn file_extension(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn sibling_temporary_path(target: &Path) -> Result<PathBuf, ResourceError> {
    let parent = target.parent().ok_or(ResourceError::PathDenied)?;
    let file_name = target.file_name().ok_or(ResourceError::PathDenied)?;
    Ok(parent.join(format!(
        ".{}.resource-{:016X}.tmp",
        file_name.to_string_lossy(),
        rand::rng().next_u64(),
    )))
}
