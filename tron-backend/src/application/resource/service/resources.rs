use serde_json::Value;

use crate::{
    application::resource::{
        ResourceCommitBinary, ResourceCommitResult, ResourceCreateCommand, ResourceDeleteCommand,
        ResourceDeleteResult, ResourceFileSummary, ResourceGetQuery, ResourcePage,
        ResourcePageQuery, ResourceRecord, ResourceUpdateCommand,
    },
    domain::resource::{
        ManagedResource, ResourceError, ResourceKind, ResourceSourceKind, validate_resource_code,
    },
    shared::{
        event_bus::{BusActorRef, BusEventDraft},
        time::current_timestamp,
    },
};

use super::ResourceService;

impl ResourceService {
    pub fn query_resources(
        &self,
        actor_user_code: &str,
        query: ResourcePageQuery,
    ) -> Result<ResourcePage, ResourceError> {
        self.query_resource_records(actor_user_code, query, false)
    }

    pub fn query_connections(
        &self,
        actor_user_code: &str,
        query: ResourcePageQuery,
    ) -> Result<ResourcePage, ResourceError> {
        self.query_resource_records(actor_user_code, query, true)
    }

    fn query_resource_records(
        &self,
        actor_user_code: &str,
        query: ResourcePageQuery,
        connections_only: bool,
    ) -> Result<ResourcePage, ResourceError> {
        let workspace_code = self.workspace_scope(actor_user_code, &query.workspace_code, false)?;
        let mut records = self
            .effective_resources(&workspace_code)?
            .into_values()
            .map(|effective| {
                self.resource_record(&effective.resource, effective.source_kind, connections_only)
            })
            .filter(|record| !connections_only || record.kind.is_connection_kind())
            .filter(|record| query.kinds.is_empty() || query.kinds.contains(&record.kind))
            .filter(|record| {
                query
                    .source_kind
                    .is_none_or(|source_kind| source_kind == record.source_kind)
            })
            .filter(|record| {
                query.search.as_deref().is_none_or(|search| {
                    let search = search.trim().to_ascii_lowercase();
                    search.is_empty()
                        || record.resource_code.to_ascii_lowercase().contains(&search)
                        || record.name.to_ascii_lowercase().contains(&search)
                        || record.description.to_ascii_lowercase().contains(&search)
                })
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.resource_code.cmp(&right.resource_code))
        });
        let total = records.len();
        let (current, size, start, end) = page_bounds(query.current, query.size, total);
        Ok(ResourcePage {
            current,
            size,
            total,
            records: records[start..end].to_vec(),
        })
    }

    pub fn get_resource(
        &self,
        actor_user_code: &str,
        query: ResourceGetQuery,
    ) -> Result<ResourceRecord, ResourceError> {
        validate_resource_code(&query.resource_code)?;
        let workspace_code = self.workspace_scope(actor_user_code, &query.workspace_code, false)?;
        let resource = self
            .effective_resources(&workspace_code)?
            .remove(&query.resource_code)
            .ok_or(ResourceError::NotFound)?;
        Ok(self.resource_record(&resource.resource, resource.source_kind, true))
    }

    pub fn create_file_resource(
        &self,
        actor_user_code: &str,
        command: ResourceCreateCommand,
        binary: Option<ResourceCommitBinary>,
    ) -> Result<ResourceCommitResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        if !command.kind.is_file_kind() {
            return Err(ResourceError::InvalidInput);
        }
        let mut binary = binary.ok_or(ResourceError::InvalidInput)?;
        if command.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        self.ensure_resource_create_is_available(&workspace_code, &command.resource_code)?;
        let record = self.store_file_resource(
            &workspace_code,
            &command.resource_code,
            command.kind,
            &command.name,
            &command.description,
            command.allow_inheritance,
            &binary.file_name,
            &mut binary.source,
        )?;
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::File(record.clone()),
            "resource.created",
        );
        Ok(ResourceCommitResult {
            resource_code: record.resource_code,
            kind: record.kind,
        })
    }

    pub fn update_resource(
        &self,
        actor_user_code: &str,
        command: ResourceUpdateCommand,
    ) -> Result<ResourceCommitResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        let kind = match self.managed_resource(&workspace_code, &command.resource_code)? {
            Some(ManagedResource::File(mut record)) => {
                apply_name(&mut record.name, command.name.as_deref())?;
                apply_description(&mut record.description, command.description.as_deref());
                apply_inheritance(&mut record.allow_inheritance, command.allow_inheritance);
                record.updated_at = current_timestamp();
                if !self.repository(|repository| repository.update_file(&record))? {
                    return Err(ResourceError::Conflict);
                }
                self.publish_resource_audit(
                    actor_user_code,
                    &ManagedResource::File(record.clone()),
                    "resource.updated",
                );
                record.kind
            }
            Some(ManagedResource::DataSourceConnection(mut record)) => {
                apply_name(&mut record.name, command.name.as_deref())?;
                apply_description(&mut record.description, command.description.as_deref());
                apply_inheritance(&mut record.allow_inheritance, command.allow_inheritance);
                record.updated_at = current_timestamp();
                if !self
                    .repository(|repository| repository.update_data_source_connection(&record))?
                {
                    return Err(ResourceError::Conflict);
                }
                self.publish_resource_audit(
                    actor_user_code,
                    &ManagedResource::DataSourceConnection(record.clone()),
                    "resource.updated",
                );
                record.kind
            }
            Some(ManagedResource::Server(mut record)) => {
                apply_name(&mut record.name, command.name.as_deref())?;
                apply_description(&mut record.description, command.description.as_deref());
                apply_inheritance(&mut record.allow_inheritance, command.allow_inheritance);
                record.updated_at = current_timestamp();
                if !self.repository(|repository| repository.update_server(&record))? {
                    return Err(ResourceError::Conflict);
                }
                self.publish_resource_audit(
                    actor_user_code,
                    &ManagedResource::Server(record.clone()),
                    "resource.updated",
                );
                ResourceKind::ServerConnection
            }
            Some(ManagedResource::ServerAccount(mut record)) => {
                apply_name(&mut record.name, command.name.as_deref())?;
                apply_description(&mut record.description, command.description.as_deref());
                apply_inheritance(&mut record.allow_inheritance, command.allow_inheritance);
                record.updated_at = current_timestamp();
                if !self.repository(|repository| repository.update_server_account(&record))? {
                    return Err(ResourceError::Conflict);
                }
                self.publish_resource_audit(
                    actor_user_code,
                    &ManagedResource::ServerAccount(record.clone()),
                    "resource.updated",
                );
                ResourceKind::ServerAccount
            }
            None => return Err(ResourceError::NotFound),
        };
        Ok(ResourceCommitResult {
            resource_code: command.resource_code,
            kind,
        })
    }

    pub fn delete_resource(
        &self,
        actor_user_code: &str,
        command: ResourceDeleteCommand,
    ) -> Result<ResourceDeleteResult, ResourceError> {
        self.delete_owned_resource(actor_user_code, command, false)
    }

    pub fn delete_connection(
        &self,
        actor_user_code: &str,
        command: ResourceDeleteCommand,
    ) -> Result<ResourceDeleteResult, ResourceError> {
        self.delete_owned_resource(actor_user_code, command, true)
    }

    fn delete_owned_resource(
        &self,
        actor_user_code: &str,
        command: ResourceDeleteCommand,
        connections_only: bool,
    ) -> Result<ResourceDeleteResult, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        let resource = self
            .managed_resource(&workspace_code, &command.resource_code)?
            .ok_or(ResourceError::NotFound)?;
        if connections_only && !resource.kind().is_connection_kind() {
            return Err(ResourceError::InvalidInput);
        }
        let deleted = match resource {
            ManagedResource::File(record) => {
                let deleted = self.repository(|repository| {
                    repository.delete_file(&workspace_code, &command.resource_code, record.revision)
                })?;
                if !deleted {
                    return Err(ResourceError::Conflict);
                }
                self.delete_unreferenced_locator(&record.storage_locator);
                true
            }
            ManagedResource::DataSourceConnection(record) => {
                if !self.repository(|repository| {
                    repository.delete_data_source_connection(
                        &workspace_code,
                        &command.resource_code,
                        record.revision,
                    )
                })? {
                    return Err(ResourceError::Conflict);
                }
                true
            }
            ManagedResource::Server(record) => {
                if !self.repository(|repository| {
                    repository.delete_server_if_no_accounts(
                        &workspace_code,
                        &command.resource_code,
                        record.revision,
                    )
                })? {
                    return Err(ResourceError::Conflict);
                }
                true
            }
            ManagedResource::ServerAccount(record) => {
                if !self.repository(|repository| {
                    repository.delete_server_account(
                        &workspace_code,
                        &command.resource_code,
                        record.revision,
                    )
                })? {
                    return Err(ResourceError::Conflict);
                }
                true
            }
        };
        self.publish_resource_audit_code(
            actor_user_code,
            &workspace_code,
            &command.resource_code,
            "resource.deleted",
        );
        Ok(ResourceDeleteResult {
            resource_code: command.resource_code,
            deleted,
        })
    }

    pub(super) fn resource_record(
        &self,
        resource: &ManagedResource,
        source_kind: ResourceSourceKind,
        include_attributes: bool,
    ) -> ResourceRecord {
        match resource {
            ManagedResource::File(record) => ResourceRecord {
                workspace_code: record.workspace_code.clone(),
                source_kind,
                resource_code: record.resource_code.clone(),
                kind: record.kind,
                name: record.name.clone(),
                description: record.description.clone(),
                allow_inheritance: record.allow_inheritance,
                file: Some(ResourceFileSummary {
                    file_name: record.file_name.clone(),
                    file_extension: record.file_extension.clone(),
                    byte_size: record.byte_size,
                }),
                attributes: None,
                created_at: record.created_at.clone(),
                updated_at: record.updated_at.clone(),
            },
            ManagedResource::DataSourceConnection(record) => ResourceRecord {
                workspace_code: record.workspace_code.clone(),
                source_kind,
                resource_code: record.resource_code.clone(),
                kind: record.kind,
                name: record.name.clone(),
                description: record.description.clone(),
                allow_inheritance: record.allow_inheritance,
                file: None,
                attributes: include_attributes.then(|| redact_direct_secrets(&record.attributes)),
                created_at: record.created_at.clone(),
                updated_at: record.updated_at.clone(),
            },
            ManagedResource::Server(record) => ResourceRecord {
                workspace_code: record.workspace_code.clone(),
                source_kind,
                resource_code: record.resource_code.clone(),
                kind: ResourceKind::ServerConnection,
                name: record.name.clone(),
                description: record.description.clone(),
                allow_inheritance: record.allow_inheritance,
                file: None,
                attributes: include_attributes.then(|| {
                    serde_json::json!({
                        "host": record.host,
                        "port": record.port,
                        "protocol": "Ssh",
                    })
                }),
                created_at: record.created_at.clone(),
                updated_at: record.updated_at.clone(),
            },
            ManagedResource::ServerAccount(record) => ResourceRecord {
                workspace_code: record.workspace_code.clone(),
                source_kind,
                resource_code: record.resource_code.clone(),
                kind: ResourceKind::ServerAccount,
                name: record.name.clone(),
                description: record.description.clone(),
                allow_inheritance: record.allow_inheritance,
                file: None,
                attributes: include_attributes.then(|| {
                    serde_json::json!({
                        "serverWorkspaceCode": record.server_workspace_code,
                        "serverCode": record.server_code,
                        "username": record.username,
                        "authenticationType": record.authentication_type,
                    })
                }),
                created_at: record.created_at.clone(),
                updated_at: record.updated_at.clone(),
            },
        }
    }

    pub(super) fn publish_resource_audit(
        &self,
        actor_user_code: &str,
        resource: &ManagedResource,
        event: &'static str,
    ) {
        self.publish_resource_audit_code(
            actor_user_code,
            resource.workspace_code(),
            resource.resource_code(),
            event,
        );
    }

    fn publish_resource_audit_code(
        &self,
        actor_user_code: &str,
        workspace_code: &str,
        resource_code: &str,
        event: &'static str,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event, "resource_service")
                .subject("resource", resource_code.to_string())
                .actor(BusActorRef {
                    user_code: actor_user_code.to_string(),
                    username: actor_user_code.to_string(),
                })
                .summary("Resource changed")
                .attr("workspaceCode", workspace_code.to_string()),
        );
    }
}

fn redact_direct_secrets(attributes: &Value) -> Value {
    let mut redacted = attributes.clone();
    redact_direct_secrets_in_place(&mut redacted);
    redacted
}

fn redact_direct_secrets_in_place(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                redact_direct_secrets_in_place(value);
            }
        }
        Value::Object(values) => {
            let is_direct = values
                .get("source")
                .and_then(Value::as_str)
                .is_some_and(|source| source == "DIRECT");
            if is_direct && values.contains_key("value") {
                values.insert("value".to_string(), Value::Null);
            }
            for value in values.values_mut() {
                redact_direct_secrets_in_place(value);
            }
        }
        _ => {}
    }
}

fn apply_name(target: &mut String, value: Option<&str>) -> Result<(), ResourceError> {
    if let Some(value) = value {
        let value = value.trim();
        if value.is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        *target = value.to_string();
    }
    Ok(())
}

fn apply_description(target: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        *target = value.trim().to_string();
    }
}

fn apply_inheritance(target: &mut bool, value: Option<bool>) {
    if let Some(value) = value {
        *target = value;
    }
}

fn page_bounds(current: usize, size: usize, total: usize) -> (usize, usize, usize, usize) {
    let current = current.max(1);
    let size = size.clamp(1, 50);
    let start = current.saturating_sub(1).saturating_mul(size).min(total);
    let end = start.saturating_add(size).min(total);
    (current, size, start, end)
}
