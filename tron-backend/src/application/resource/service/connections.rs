use crate::{
    application::resource::{
        DataSourceCreateCommand, DataSourceUpdateCommand, ResourceCommitOutcome,
        ServerAccountCreateCommand, ServerCreateCommand, ServerCreateResult,
    },
    domain::resource::{
        DataSourceConnectionRecord, ManagedResource, ResourceError, ServerAccountRecord,
        ServerRecord, validate_resource_code,
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
        let target = self.data_source_connection_target(
            &workspace_code,
            command.kind,
            &command.attributes,
        )?;
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
            let target = self.data_source_connection_target(
                &workspace_code,
                record.kind,
                attributes,
            )?;
            self.connection_verifier.verify(target).await?;
            record.attributes = attributes.clone();
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
        self.repository(|repository| repository.update_data_source_connection(&record))?;
        Ok(commit_outcome(&workspace_code, &record.resource_code, record.kind))
    }

    pub async fn create_server_for_http(
        &self,
        actor_user_code: &str,
        command: ServerCreateCommand,
    ) -> Result<ServerCreateResult, ResourceError> {
        validate_resource_code(&command.server_code)?;
        validate_resource_code(&command.initial_account.account_code)?;
        if command.server_code == command.initial_account.account_code {
            return Err(ResourceError::InvalidInput);
        }
        if command.name.trim().is_empty() || command.initial_account.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        self.ensure_resource_create_is_available(&workspace_code, &command.server_code)?;
        self.ensure_resource_create_is_available(
            &workspace_code,
            &command.initial_account.account_code,
        )?;
        validate_account_credentials(&command.initial_account)?;
        let target = self.ssh_connection_target(
            &workspace_code,
            command.host.clone(),
            command.port,
            &command.initial_account,
        )?;
        self.connection_verifier.verify(target).await?;
        let now = current_timestamp();
        let server = ServerRecord {
            workspace_code: workspace_code.clone(),
            resource_code: command.server_code.clone(),
            name: command.name.trim().to_string(),
            description: command.description.trim().to_string(),
            host: command.host.trim().to_string(),
            port: command.port,
            allow_inheritance: command.allow_inheritance,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let account = server_account_record(
            &workspace_code,
            &command.server_code,
            &command.initial_account,
            &now,
        )?;
        self.repository(|repository| repository.insert_server(&server))?;
        if let Err(error) = self.repository(|repository| repository.insert_server_account(&account)) {
            let _ = self
                .repository(|repository| repository.delete_server(&workspace_code, &command.server_code));
            return Err(error);
        }
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::Server(server.clone()),
            "resource.created",
        );
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::ServerAccount(account.clone()),
            "resource.created",
        );
        Ok(ServerCreateResult {
            server: commit_outcome(
                &workspace_code,
                &server.resource_code,
                crate::domain::resource::ResourceKind::ServerConnection,
            ),
            initial_account: commit_outcome(
                &workspace_code,
                &account.resource_code,
                crate::domain::resource::ResourceKind::ServerAccount,
            ),
            connectivity_verified: true,
        })
    }

    pub fn create_server_account_for_http(
        &self,
        actor_user_code: &str,
        command: ServerAccountCreateCommand,
    ) -> Result<ResourceCommitOutcome, ResourceError> {
        validate_resource_code(&command.server_code)?;
        validate_resource_code(&command.account.account_code)?;
        if command.account.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        let workspace_code =
            self.workspace_scope(actor_user_code, &command.workspace_code, true)?;
        let server = self
            .managed_resource(&workspace_code, &command.server_code)?
            .ok_or(ResourceError::NotFound)?;
        if !matches!(server, ManagedResource::Server(_)) {
            return Err(ResourceError::NotFound);
        }
        self.ensure_resource_create_is_available(&workspace_code, &command.account.account_code)?;
        validate_account_credentials(&command.account)?;
        let now = current_timestamp();
        let account = server_account_record(
            &workspace_code,
            &command.server_code,
            &command.account,
            &now,
        )?;
        self.repository(|repository| repository.insert_server_account(&account))?;
        self.publish_resource_audit(
            actor_user_code,
            &ManagedResource::ServerAccount(account.clone()),
            "resource.created",
        );
        Ok(commit_outcome(
            &workspace_code,
            &account.resource_code,
            crate::domain::resource::ResourceKind::ServerAccount,
        ))
    }

    pub(super) fn ensure_server_has_no_accounts(
        &self,
        workspace_code: &str,
        server_code: &str,
    ) -> Result<(), ResourceError> {
        let has_accounts = self
            .repository(|repository| repository.list_server_accounts())?
            .iter()
            .any(|account| {
                account.workspace_code == workspace_code && account.server_code == server_code
            });
        if has_accounts {
            Err(ResourceError::ServerHasAccounts)
        } else {
            Ok(())
        }
    }
}

fn server_account_record(
    workspace_code: &str,
    server_code: &str,
    input: &crate::application::resource::ServerAccountCreateInput,
    now: &str,
) -> Result<ServerAccountRecord, ResourceError> {
    Ok(ServerAccountRecord {
        workspace_code: workspace_code.to_string(),
        resource_code: input.account_code.clone(),
        server_code: server_code.to_string(),
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        username: input.username.trim().to_string(),
        authentication_type: input.authentication_type,
        password_variable_key: input.password_variable_key.clone(),
        private_key_variable_key: input.private_key_variable_key.clone(),
        private_key_passphrase_variable_key: input.private_key_passphrase_variable_key.clone(),
        allow_inheritance: input.allow_inheritance,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    })
}

fn validate_account_credentials(
    input: &crate::application::resource::ServerAccountCreateInput,
) -> Result<(), ResourceError> {
    if input.username.trim().is_empty() {
        return Err(ResourceError::InvalidInput);
    }
    match input.authentication_type {
        crate::domain::resource::ServerAuthenticationType::Password => {
            if input.password_variable_key.is_none()
                || input.private_key_variable_key.is_some()
                || input.private_key_passphrase_variable_key.is_some()
            {
                return Err(ResourceError::InvalidInput);
            }
        }
        crate::domain::resource::ServerAuthenticationType::PrivateKey => {
            if input.private_key_variable_key.is_none() || input.password_variable_key.is_some() {
                return Err(ResourceError::InvalidInput);
            }
        }
    }
    Ok(())
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
