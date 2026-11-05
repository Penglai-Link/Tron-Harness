use serde_json::Value;

use crate::{
    application::resource::{
        DataSourceCreateCommand, DataSourceUpdateCommand, ResourceCommitOutcome,
        ServerCreateCommand, ServerCreateResult,
    },
    domain::resource::{
        DataSourceConnectionRecord, ManagedResource, ResourceError, ServerRecord,
        validate_resource_code,
    },
    shared::time::current_timestamp,
};

use super::ResourceService;

impl ResourceService {
    pub async fn create_data_source_for_http(
        &self,
        actor_user_code: &str,
        command: DataSourceCreateCommand,
    ) -> Result<ResourceCommitOutcome, ResourceError> {
        if !command.kind.is_data_source() {
            return Err(ResourceError::InvalidInput);
        }
        if command.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        validate_resource_code(&command.resource_code)?;
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        self.ensure_resource_create_is_available(&workspace_code, &command.resource_code)?;
        let target =
            self.data_source_connection_target(&workspace_code, command.kind, &command.attributes)?;
        self.connection_verifier.verify(target).await?;
        let now = current_timestamp();
        let record = DataSourceConnectionRecord {
            workspace_code: workspace_code.clone(),
            resource_code: command.resource_code.clone(),
            kind: command.kind,
            name: command.name.trim().to_string(),
            description: command.description.trim().to_string(),
            attributes: command.attributes,
            allow_inheritance: command.allow_inheritance,
            revision: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repository(|repository| repository.insert_data_source_connection(&record))?;
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::DataSourceConnection(record.clone()),
            "resource.created",
        );
        Ok(commit_outcome(
            &workspace_code,
            &record.resource_code,
            record.kind,
        ))
    }

    pub async fn update_data_source_for_http(
        &self,
        actor_user_code: &str,
        command: DataSourceUpdateCommand,
    ) -> Result<ResourceCommitOutcome, ResourceError> {
        validate_resource_code(&command.resource_code)?;
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        let mut record = match self.repository(|repository| {
            Ok(repository
                .list_data_source_connections()?
                .into_iter()
                .find(|record| {
                    record.workspace_code == workspace_code
                        && record.resource_code == command.resource_code
                }))
        })? {
            Some(record) => record,
            None => return Err(ResourceError::NotFound),
        };
        if let Some(attributes) = &command.attributes {
            let attributes = merge_redacted_direct_secrets(&record.attributes, attributes)?;
            let target =
                self.data_source_connection_target(&workspace_code, record.kind, &attributes)?;
            self.connection_verifier.verify(target).await?;
            record.attributes = attributes;
        }
        if let Some(name) = &command.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(ResourceError::InvalidInput);
            }
            record.name = name.to_string();
        }
        if let Some(description) = &command.description {
            record.description = description.trim().to_string();
        }
        if let Some(allow_inheritance) = command.allow_inheritance {
            record.allow_inheritance = allow_inheritance;
        }
        record.updated_at = current_timestamp();
        if !self.repository(|repository| repository.update_data_source_connection(&record))? {
            return Err(ResourceError::Conflict);
        }
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::DataSourceConnection(record.clone()),
            "resource.updated",
        );
        Ok(commit_outcome(
            &workspace_code,
            &record.resource_code,
            record.kind,
        ))
    }

    pub async fn create_server_for_http(
        &self,
        actor_user_code: &str,
        command: ServerCreateCommand,
    ) -> Result<ServerCreateResult, ResourceError> {
        validate_resource_code(&command.server_code)?;
        if command.name.trim().is_empty() || command.host.trim().is_empty() || command.port == 0 {
            return Err(ResourceError::InvalidInput);
        }
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        self.ensure_resource_create_is_available(&workspace_code, &command.server_code)?;
        let now = current_timestamp();
        let server = ServerRecord {
            workspace_code: workspace_code.clone(),
            resource_code: command.server_code.clone(),
            name: command.name.trim().to_string(),
            description: command.description.trim().to_string(),
            host: command.host.trim().to_string(),
            port: command.port,
            allow_inheritance: command.allow_inheritance,
            revision: 0,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let account = match command.initial_account {
            Some(initial_account) => {
                validate_resource_code(&initial_account.account_code)?;
                if command.server_code == initial_account.account_code {
                    return Err(ResourceError::InvalidInput);
                }
                if initial_account.name.trim().is_empty() {
                    return Err(ResourceError::InvalidInput);
                }
                self.ensure_resource_create_is_available(
                    &workspace_code,
                    &initial_account.account_code,
                )?;
                super::server_accounts::validate_account_credentials(&initial_account)?;
                let target = self.ssh_connection_target(
                    command.host.clone(),
                    command.port,
                    &initial_account,
                )?;
                self.connection_verifier.verify(target).await?;
                Some(self.server_account_record(
                    &workspace_code,
                    &initial_account,
                    vec![crate::domain::resource::ServerAccountServer {
                        workspace_code: server.workspace_code.clone(),
                        server_code: command.server_code.clone(),
                    }],
                    &now,
                )?)
            }
            None => None,
        };
        if let Some(account) = account.as_ref() {
            self.repository(|repository| repository.insert_server_bundle(&server, account))?;
        } else {
            self.repository(|repository| repository.insert_server(&server))?;
        }
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::Server(server.clone()),
            "resource.created",
        );
        if let Some(account) = account.as_ref() {
            self.publish_resource_audit(
                actor_user_code,
                &ManagedResource::ServerAccount(account.clone()),
                "resource.created",
            );
        }
        let connectivity_verified = account.is_some();
        Ok(ServerCreateResult {
            server: commit_outcome(
                &workspace_code,
                &server.resource_code,
                crate::domain::resource::ResourceKind::ServerConnection,
            ),
            initial_account: account.map(|account| {
                commit_outcome(
                    &workspace_code,
                    &account.resource_code,
                    crate::domain::resource::ResourceKind::ServerAccount,
                )
            }),
            connectivity_verified,
        })
    }
}

fn merge_redacted_direct_secrets(
    existing: &Value,
    replacement: &Value,
) -> Result<Value, ResourceError> {
    let mut replacement = replacement.clone();
    merge_redacted_direct_secrets_in_place(existing, &mut replacement)?;
    Ok(replacement)
}

fn merge_redacted_direct_secrets_in_place(
    existing: &Value,
    replacement: &mut Value,
) -> Result<(), ResourceError> {
    match (existing, replacement) {
        (Value::Object(existing), Value::Object(replacement)) => {
            let is_redacted_direct = replacement
                .get("source")
                .and_then(Value::as_str)
                .is_some_and(|source| source == "DIRECT")
                && replacement.get("value").is_some_and(Value::is_null);
            if is_redacted_direct {
                let source_matches = existing
                    .get("source")
                    .and_then(Value::as_str)
                    .is_some_and(|source| source == "DIRECT");
                let value = existing.get("value").and_then(Value::as_str);
                if !source_matches || value.is_none() {
                    return Err(ResourceError::InvalidInput);
                }
                replacement.insert(
                    "value".to_string(),
                    Value::String(value.unwrap().to_string()),
                );
            }
            for (key, replacement_value) in replacement.iter_mut() {
                if let Some(existing_value) = existing.get(key) {
                    merge_redacted_direct_secrets_in_place(existing_value, replacement_value)?;
                }
            }
            Ok(())
        }
        (Value::Array(existing), Value::Array(replacement)) => {
            for (existing_value, replacement_value) in existing.iter().zip(replacement.iter_mut()) {
                merge_redacted_direct_secrets_in_place(existing_value, replacement_value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn commit_outcome(
    workspace_code: &str,
    resource_code: &str,
    kind: crate::domain::resource::ResourceKind,
) -> ResourceCommitOutcome {
    ResourceCommitOutcome {
        workspace_code: workspace_code.to_string(),
        resource_code: resource_code.to_string(),
        kind,
    }
}
