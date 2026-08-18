use crate::{
    application::resource::{
        DataSourceCreateCommand, ResourceActivationOutput, ResourceCommitCommand,
        ServerAccountCreateCommand, ServerCreateCommand, ServerCreateResult,
    },
    domain::resource::{
        Resource, ResourceActivationResult, ResourceError, ResourceKey, ResourceLifecycleStatus,
        ResourceScopeKind, ResourceValidationStatus, SERVER_ACCOUNT_DEFINITION_CODE,
        SERVER_CONNECTION_DEFINITION_CODE, is_data_source_definition_code, validate_resource_code,
    },
    shared::time::current_timestamp,
};

use super::{
    ResourceService,
    connection_attributes::{server_account_attributes, server_attributes},
    resources::next_resource_version,
};

struct ConnectionResourceInput<'a> {
    resource_code: &'a str,
    definition_code: &'a str,
    name: &'a str,
    description: &'a str,
    attributes: serde_json::Value,
    allow_inheritance: bool,
}

impl ResourceService {
    pub async fn create_data_source_for_http(
        &self,
        actor_user_code: &str,
        command: DataSourceCreateCommand,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        if !is_data_source_definition_code(&command.definition_code) {
            return Err(ResourceError::InvalidInput);
        }
        let commit = ResourceCommitCommand {
            scope_kind: command.scope_kind,
            workspace_code: command.workspace_code,
            resource_code: command.resource_code,
            definition_code: Some(command.definition_code.clone()),
            expected_active_resource_version: None,
            name: command.name,
            description: command.description,
            attributes: command.attributes,
            allow_inheritance: command.allow_inheritance,
            removed_binary_names: Vec::new(),
        };
        self.preflight_verified_connection_commit(actor_user_code, &commit)?;
        let scope_code = self.management_scope(
            actor_user_code,
            commit.scope_kind,
            commit.workspace_code.as_deref(),
            true,
        )?;
        let resource = self.prepare_active_connection_resource(
            commit.scope_kind,
            &scope_code,
            ConnectionResourceInput {
                resource_code: &commit.resource_code,
                definition_code: &command.definition_code,
                name: &commit.name,
                description: &commit.description,
                attributes: commit.attributes.clone(),
                allow_inheritance: commit.allow_inheritance,
            },
        )?;
        let target = self.data_source_connection_target(
            commit.scope_kind,
            &scope_code,
            &command.definition_code,
            &commit.attributes,
        )?;
        self.connection_verifier.verify(target).await?;
        self.persist_active_connection_resources(actor_user_code, &scope_code, &[resource])
            .map(|mut values| values.remove(0))
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
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let server = self.prepare_active_connection_resource(
            command.scope_kind,
            &scope_code,
            ConnectionResourceInput {
                resource_code: &command.server_code,
                definition_code: SERVER_CONNECTION_DEFINITION_CODE,
                name: &command.name,
                description: &command.description,
                attributes: server_attributes(&command.host, command.port),
                allow_inheritance: command.allow_inheritance,
            },
        )?;
        let account = self.prepare_active_connection_resource(
            command.scope_kind,
            &scope_code,
            ConnectionResourceInput {
                resource_code: &command.initial_account.account_code,
                definition_code: SERVER_ACCOUNT_DEFINITION_CODE,
                name: &command.initial_account.name,
                description: &command.initial_account.description,
                attributes: server_account_attributes(
                    &command.server_code,
                    &command.initial_account,
                ),
                allow_inheritance: command.initial_account.allow_inheritance,
            },
        )?;
        let target = self.ssh_connection_target(
            command.scope_kind,
            &scope_code,
            command.host,
            command.port,
            &command.initial_account,
        )?;
        self.connection_verifier.verify(target).await?;
        let mut created = self.persist_active_connection_resources(
            actor_user_code,
            &scope_code,
            &[server, account],
        )?;
        Ok(ServerCreateResult {
            server: created.remove(0),
            initial_account: created.remove(0),
            connectivity_verified: true,
        })
    }

    pub fn create_server_account_for_http(
        &self,
        actor_user_code: &str,
        command: ServerAccountCreateCommand,
    ) -> Result<ResourceActivationOutput, ResourceError> {
        validate_resource_code(&command.server_code)?;
        validate_resource_code(&command.account.account_code)?;
        let scope_code = self.management_scope(
            actor_user_code,
            command.scope_kind,
            command.workspace_code.as_deref(),
            true,
        )?;
        let server = self.effective_active_resource_for_http(
            command.scope_kind,
            &scope_code,
            &command.server_code,
        )?;
        if server.definition_key.definition_code != SERVER_CONNECTION_DEFINITION_CODE {
            return Err(ResourceError::NotFound);
        }
        let account = self.prepare_active_connection_resource(
            command.scope_kind,
            &scope_code,
            ConnectionResourceInput {
                resource_code: &command.account.account_code,
                definition_code: SERVER_ACCOUNT_DEFINITION_CODE,
                name: &command.account.name,
                description: &command.account.description,
                attributes: server_account_attributes(&command.server_code, &command.account),
                allow_inheritance: command.account.allow_inheritance,
            },
        )?;
        self.persist_active_connection_resources(actor_user_code, &scope_code, &[account])
            .map(|mut values| values.remove(0))
    }

    fn prepare_active_connection_resource(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        input: ConnectionResourceInput<'_>,
    ) -> Result<Resource, ResourceError> {
        validate_resource_code(input.resource_code)?;
        if input.name.trim().is_empty() {
            return Err(ResourceError::InvalidInput);
        }
        self.ensure_resource_create_is_available_for_scope(
            scope_kind,
            scope_code,
            input.resource_code,
        )?;
        let definition =
            self.definition_for_resource_scope(scope_kind, scope_code, input.definition_code)?;
        let all_resources = self.repository(|repository| repository.list_resources())?;
        let now = current_timestamp();
        let resource = Resource {
            key: ResourceKey {
                scope_kind,
                scope_code: scope_code.to_string(),
                resource_code: input.resource_code.to_string(),
                resource_version: next_resource_version(
                    &all_resources,
                    scope_kind,
                    scope_code,
                    input.resource_code,
                )?,
            },
            definition_key: definition.key.clone(),
            based_on_resource_version: None,
            name: input.name.trim().to_string(),
            description: input.description.trim().to_string(),
            attributes: input.attributes,
            allow_inheritance: input.allow_inheritance,
            draft_revision: 0,
            validation_status: ResourceValidationStatus::Valid,
            lifecycle_status: ResourceLifecycleStatus::Active,
            created_at: now.clone(),
            updated_at: now.clone(),
            published_at: Some(now),
        };
        self.ensure_resource_publishable(&resource, &definition, &[])?;
        Ok(resource)
    }

    fn persist_active_connection_resources(
        &self,
        actor_user_code: &str,
        scope_code: &str,
        resources: &[Resource],
    ) -> Result<Vec<ResourceActivationOutput>, ResourceError> {
        self.repository(|repository| repository.insert_active_resource_bundle(resources))?;
        for resource in resources {
            self.publish_resource_audit(
                actor_user_code,
                resource,
                scope_code,
                ResourceActivationResult::DirectCommitted,
                None,
            );
        }
        Ok(resources.iter().map(activation_output).collect())
    }

    pub(super) fn ensure_server_has_no_accounts(
        &self,
        scope_kind: ResourceScopeKind,
        scope_code: &str,
        server_code: &str,
    ) -> Result<(), ResourceError> {
        let resources = self.repository(|repository| repository.list_resources())?;
        let is_server = resources.iter().any(|resource| {
            resource.key.scope_kind == scope_kind
                && resource.key.scope_code == scope_code
                && resource.key.resource_code == server_code
                && resource.definition_key.definition_code == SERVER_CONNECTION_DEFINITION_CODE
                && resource.lifecycle_status == ResourceLifecycleStatus::Active
        });
        if !is_server {
            return Ok(());
        }
        let has_accounts = resources.iter().any(|resource| {
            resource.key.scope_kind == scope_kind
                && resource.key.scope_code == scope_code
                && resource.definition_key.definition_code == SERVER_ACCOUNT_DEFINITION_CODE
                && resource.lifecycle_status != ResourceLifecycleStatus::Historical
                && resource
                    .attributes
                    .get("serverCode")
                    .and_then(serde_json::Value::as_str)
                    == Some(server_code)
        });
        if has_accounts {
            Err(ResourceError::ServerHasAccounts)
        } else {
            Ok(())
        }
    }
}

fn activation_output(resource: &Resource) -> ResourceActivationOutput {
    ResourceActivationOutput {
        resource_code: resource.key.resource_code.clone(),
        resource_version: resource.key.resource_version.clone(),
        definition_scope_kind: resource.definition_key.scope_kind,
        definition_scope_code: resource.definition_key.scope_code.clone(),
        definition_code: resource.definition_key.definition_code.clone(),
        definition_version: resource.definition_key.definition_version.clone(),
        lifecycle_status: ResourceLifecycleStatus::Active,
        activation_result: ResourceActivationResult::DirectCommitted,
        previous_active_resource_version: None,
    }
}
