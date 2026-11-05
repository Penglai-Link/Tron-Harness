use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    application::resource::{
        ResourceCommitOutcome, ServerAccountAssignCommand, ServerAccountCreateCommand,
        ServerAccountCreateInput, ServerAccountUpdateCommand, ServerAccountVerifyCommand,
    },
    domain::{
        resource::{
            ManagedResource, ResourceError, ResourceKind, ServerAccountRecord, ServerAccountServer,
            ServerAuthenticationType, validate_resource_code,
        },
        variable::EncryptedVariableValue,
    },
    shared::time::current_timestamp,
};

use super::ResourceService;

// This payload only crosses the encryption boundary. Never return or log it.
#[derive(Serialize, Deserialize)]
struct AccountCredentials {
    password: Option<String>,
    private_key: Option<String>,
    private_key_passphrase: Option<String>,
}

impl ResourceService {
    pub fn create_server_account_for_http(
        &self,
        actor: &str,
        command: ServerAccountCreateCommand,
    ) -> Result<ResourceCommitOutcome, ResourceError> {
        let workspace = self.workspace_scope(actor, &command.workspace_code, true)?;
        validate_resource_code(&command.account.account_code)?;
        self.ensure_resource_create_is_available(&workspace, &command.account.account_code)?;
        let account = self.server_account_record(
            &workspace,
            &command.account,
            Vec::new(),
            &current_timestamp(),
        )?;
        self.repository(|repository| repository.insert_server_account(&account))?;
        self.publish_resource_audit(
            actor,
            &ManagedResource::ServerAccount(account.clone()),
            "resource.created",
        );
        Ok(outcome(&account))
    }

    pub fn update_server_account_for_http(
        &self,
        actor: &str,
        command: ServerAccountUpdateCommand,
    ) -> Result<ResourceCommitOutcome, ResourceError> {
        let mut account =
            self.writable_server_account(actor, &command.workspace_code, &command.account_code)?;
        if command.name.trim().is_empty() || command.username.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        let replace_credentials = command.authentication_type != account.authentication_type
            || command.password.is_some()
            || command.private_key.is_some()
            || command.private_key_passphrase.is_some();
        if replace_credentials {
            let input = ServerAccountCreateInput {
                account_code: account.resource_code.clone(),
                name: command.name.clone(),
                description: command.description.clone(),
                username: command.username.clone(),
                authentication_type: command.authentication_type,
                password: command.password,
                private_key: command.private_key,
                private_key_passphrase: command.private_key_passphrase,
                allow_inheritance: command.allow_inheritance,
            };
            validate_account_credentials(&input)?;
            account.encrypted_credentials =
                self.encrypt_account_credentials(&account.workspace_code, &input)?;
        }
        account.name = command.name.trim().to_owned();
        account.description = command.description.trim().to_owned();
        account.username = command.username.trim().to_owned();
        account.authentication_type = command.authentication_type;
        account.allow_inheritance = command.allow_inheritance;
        account.updated_at = current_timestamp();
        self.save_server_account(actor, &account)?;
        Ok(outcome(&account))
    }

    pub fn assign_server_account_for_http(
        &self,
        actor: &str,
        command: ServerAccountAssignCommand,
    ) -> Result<ResourceCommitOutcome, ResourceError> {
        let mut account =
            self.writable_server_account(actor, &command.workspace_code, &command.account_code)?;
        if command.server_codes.len() > 500 {
            return Err(ResourceError::InvalidInput);
        }
        let effective = self.effective_resources(&account.workspace_code)?;
        let mut servers = Vec::new();
        for server_code in command.server_codes.into_iter().collect::<BTreeSet<_>>() {
            validate_resource_code(&server_code)?;
            let server = match effective.get(&server_code).map(|entry| &entry.resource) {
                Some(ManagedResource::Server(server)) => server,
                _ => return Err(ResourceError::NotFound),
            };
            self.authorize_resource_read(actor, &server.workspace_code, &server.resource_code)?;
            servers.push(ServerAccountServer {
                workspace_code: server.workspace_code.clone(),
                server_code: server.resource_code.clone(),
            });
        }
        account.servers = servers;
        account.updated_at = current_timestamp();
        self.save_server_account(actor, &account)?;
        Ok(outcome(&account))
    }

    pub async fn verify_server_account_for_http(
        &self,
        actor: &str,
        command: ServerAccountVerifyCommand,
    ) -> Result<(), ResourceError> {
        let workspace = self.workspace_scope(actor, &command.workspace_code, false)?;
        let effective = self.effective_resources(&workspace)?;
        let account = match effective
            .get(&command.account_code)
            .map(|entry| &entry.resource)
        {
            Some(ManagedResource::ServerAccount(account)) => account,
            _ => return Err(ResourceError::NotFound),
        };
        let server = match effective
            .get(&command.server_code)
            .map(|entry| &entry.resource)
        {
            Some(ManagedResource::Server(server)) => server,
            _ => return Err(ResourceError::NotFound),
        };
        self.authorize_resource_read(actor, &account.workspace_code, &account.resource_code)?;
        self.authorize_resource_read(actor, &server.workspace_code, &server.resource_code)?;
        if !account.servers.iter().any(|binding| {
            binding.workspace_code == server.workspace_code
                && binding.server_code == server.resource_code
        }) {
            return Err(ResourceError::InvalidInput);
        }
        // Only the backend connection use case decrypts stored credentials.
        let encrypted: EncryptedVariableValue =
            serde_json::from_str(&account.encrypted_credentials)
                .map_err(|_| ResourceError::StorageFailure)?;
        let plaintext = self
            .variables
            .cipher()
            .decrypt(
                &encrypted,
                credential_aad(&account.workspace_code, &account.resource_code).as_bytes(),
            )
            .map_err(|_| ResourceError::StorageFailure)?;
        let credentials: AccountCredentials =
            serde_json::from_str(&plaintext).map_err(|_| ResourceError::StorageFailure)?;
        let input = ServerAccountCreateInput {
            account_code: account.resource_code.clone(),
            name: account.name.clone(),
            description: account.description.clone(),
            username: account.username.clone(),
            authentication_type: account.authentication_type,
            password: credentials.password,
            private_key: credentials.private_key,
            private_key_passphrase: credentials.private_key_passphrase,
            allow_inheritance: account.allow_inheritance,
        };
        let target = self.ssh_connection_target(server.host.clone(), server.port, &input)?;
        self.connection_verifier.verify(target).await
    }

    pub(super) fn server_account_record(
        &self,
        workspace: &str,
        input: &ServerAccountCreateInput,
        servers: Vec<ServerAccountServer>,
        now: &str,
    ) -> Result<ServerAccountRecord, ResourceError> {
        validate_account_credentials(input)?;
        Ok(ServerAccountRecord {
            workspace_code: workspace.to_owned(),
            resource_code: input.account_code.clone(),
            servers,
            name: input.name.trim().to_owned(),
            description: input.description.trim().to_owned(),
            username: input.username.trim().to_owned(),
            authentication_type: input.authentication_type,
            encrypted_credentials: self.encrypt_account_credentials(workspace, input)?,
            allow_inheritance: input.allow_inheritance,
            revision: 0,
            created_at: now.to_owned(),
            updated_at: now.to_owned(),
        })
    }

    fn encrypt_account_credentials(
        &self,
        workspace: &str,
        input: &ServerAccountCreateInput,
    ) -> Result<String, ResourceError> {
        let plaintext = serde_json::to_string(&AccountCredentials {
            password: input.password.clone(),
            private_key: input.private_key.clone(),
            private_key_passphrase: input.private_key_passphrase.clone(),
        })
        .map_err(|_| ResourceError::StorageFailure)?;
        // Reuse the configured cipher, not the variable catalogue or variable records.
        let encrypted = self
            .variables
            .cipher()
            .encrypt(
                &plaintext,
                credential_aad(workspace, &input.account_code).as_bytes(),
            )
            .map_err(|_| ResourceError::StorageFailure)?;
        serde_json::to_string(&encrypted).map_err(|_| ResourceError::StorageFailure)
    }

    fn writable_server_account(
        &self,
        actor: &str,
        workspace: &str,
        code: &str,
    ) -> Result<ServerAccountRecord, ResourceError> {
        let workspace = self.workspace_scope(actor, workspace, true)?;
        validate_resource_code(code)?;
        self.authorize_resource_mutation(actor, &workspace, code)?;
        match self.managed_resource(&workspace, code)? {
            Some(ManagedResource::ServerAccount(account)) => Ok(account),
            _ => Err(ResourceError::NotFound),
        }
    }

    fn save_server_account(
        &self,
        actor: &str,
        account: &ServerAccountRecord,
    ) -> Result<(), ResourceError> {
        if !self.repository(|repository| repository.update_server_account(account))? {
            return Err(ResourceError::Conflict);
        }
        self.publish_resource_audit(
            actor,
            &ManagedResource::ServerAccount(account.clone()),
            "resource.updated",
        );
        Ok(())
    }
}

pub(super) fn validate_account_credentials(
    input: &ServerAccountCreateInput,
) -> Result<(), ResourceError> {
    if input.name.trim().is_empty() || input.username.trim().is_empty() {
        return Err(ResourceError::InvalidInput);
    }
    let valid_secret = |value: &Option<String>| {
        value
            .as_ref()
            .is_some_and(|value| !value.is_empty() && value.len() <= 65_536)
    };
    let valid = match input.authentication_type {
        ServerAuthenticationType::Password => {
            valid_secret(&input.password)
                && input.private_key.is_none()
                && input.private_key_passphrase.is_none()
        }
        ServerAuthenticationType::PrivateKey => {
            valid_secret(&input.private_key)
                && input
                    .private_key
                    .as_ref()
                    .is_some_and(|value| !value.trim().is_empty())
                && input.password.is_none()
                && input
                    .private_key_passphrase
                    .as_ref()
                    .is_none_or(|value| value.len() <= 65_536)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(ResourceError::InvalidInput)
    }
}

fn credential_aad(workspace: &str, account: &str) -> String {
    format!("resource-server-account:{workspace}:{account}")
}

fn outcome(account: &ServerAccountRecord) -> ResourceCommitOutcome {
    ResourceCommitOutcome {
        workspace_code: account.workspace_code.clone(),
        resource_code: account.resource_code.clone(),
        kind: ResourceKind::ServerAccount,
    }
}
