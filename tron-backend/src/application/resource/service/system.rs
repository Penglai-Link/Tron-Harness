use std::io::Read;

use crate::domain::resource::{
    ManagedResource, ResourceError, ResourceFileRecord, ResourceKind, validate_resource_code,
};

use super::{ResourceService, SYSTEM_RESOURCE_SCOPE};

const SYSTEM_AVATAR_CODE_PREFIX: &str = "AVATAR-";

impl ResourceService {
    pub fn replace_system_avatar(
        &self,
        actor_user_code: &str,
        file_name: &str,
        source: &mut dyn Read,
    ) -> Result<ResourceFileRecord, ResourceError> {
        let resource_code = system_avatar_resource_code(actor_user_code)?;
        if let Some(resource) = self.managed_resource(SYSTEM_RESOURCE_SCOPE, &resource_code)? {
            let old = match resource {
                ManagedResource::File(record) => record,
                _ => return Err(ResourceError::Conflict),
            };
            if !self.repository(|repository| {
                repository.delete_file(SYSTEM_RESOURCE_SCOPE, &resource_code, old.revision)
            })? {
                return Err(ResourceError::Conflict);
            }
            self.delete_unreferenced_locator(&old.storage_locator);
        }

        self.store_file_resource(
            SYSTEM_RESOURCE_SCOPE,
            &resource_code,
            ResourceKind::Image,
            "System avatar",
            "System-managed account avatar",
            false,
            file_name,
            source,
        )
    }

    pub fn download_system_avatar(
        &self,
        actor_user_code: &str,
    ) -> Result<
        (
            crate::application::resource::ResourceHttpBinaryDownload,
            ResourceFileRecord,
        ),
        ResourceError,
    > {
        let resource_code = system_avatar_resource_code(actor_user_code)?;
        let record = match self.managed_resource(SYSTEM_RESOURCE_SCOPE, &resource_code)? {
            Some(ManagedResource::File(record)) => record,
            Some(_) | None => return Err(ResourceError::NotFound),
        };
        if !self.storage.exists(&record.storage_locator)? {
            return Err(ResourceError::MissingBinaryObject);
        }
        Ok((
            crate::application::resource::ResourceHttpBinaryDownload {
                file_name: record.file_name.clone(),
                byte_size: record.byte_size,
            },
            record,
        ))
    }
}

fn system_avatar_resource_code(actor_user_code: &str) -> Result<String, ResourceError> {
    let suffix: String = actor_user_code
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let code = format!("{SYSTEM_AVATAR_CODE_PREFIX}{suffix}");
    let code = code.chars().take(64).collect::<String>();
    validate_resource_code(&code)?;
    Ok(code)
}
