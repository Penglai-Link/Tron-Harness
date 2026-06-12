use crate::{
    domain::open_app::{
        OpenApp, OpenAppCapabilityGrant, OpenAppError, OpenAppStatus, OpenAppUserGrant,
    },
    shared::{id::SnowflakeId, random::random_base64url, time::current_timestamp},
};

use super::{
    CreateOpenAppCommand, CreatedOpenApp, ManagedOpenAppDetail, ManagedOpenAppSecretReveal,
    ManagedOpenAppSecretRevealResult, ManagedOpenAppSummary, OpenAppService,
    ReplaceCapabilityGrantsCommand, ReplaceUserGrantsCommand, UpdateOpenAppCommand,
    shared::{
        build_active_credential, map_detail_from_auth_state, map_summary_from_auth_state,
        normalize_capability_grants, normalize_user_codes, optional_trimmed, required_trimmed,
    },
};
use crate::application::open_app::{
    OpenApiCapabilityCatalogGroup, list_open_api_capability_catalog, validate_open_api_grant,
};

impl OpenAppService {
    pub fn list_open_apps(&self) -> Result<Vec<ManagedOpenAppSummary>, OpenAppError> {
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let auth_states = repository.list_open_app_auth_states()?;
        Ok(auth_states
            .iter()
            .map(map_summary_from_auth_state)
            .collect::<Vec<_>>())
    }

    pub fn get_open_app_detail(
        &self,
        open_app_code: &str,
    ) -> Result<ManagedOpenAppDetail, OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(auth_state) = repository.load_auth_state(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };
        Ok(map_detail_from_auth_state(&auth_state))
    }

    pub fn create_open_app(
        &self,
        command: CreateOpenAppCommand,
    ) -> Result<CreatedOpenApp, OpenAppError> {
        let name = required_trimmed(command.name, OpenAppError::NameRequired)?;
        let description = optional_trimmed(command.description);
        let now = current_timestamp();
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let mut next_id = repository.next_id()?;
        let open_app_id = next_id;
        let open_app_code = format!("OPENAPP-{:016X}", open_app_id);
        next_id += 1;

        let app_key = random_base64url(32);
        let open_app = OpenApp {
            id: SnowflakeId::new(open_app_id),
            open_app_code: open_app_code.clone(),
            name,
            description,
            status: OpenAppStatus::Enabled,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_rotated_at: Some(now.clone()),
        };
        let credential = build_active_credential(
            &mut next_id,
            &self.secret_box,
            &app_key,
            &now,
            &open_app_code,
        )?;

        repository.create_open_app(&open_app, &credential)?;
        Ok(CreatedOpenApp {
            open_app,
            app_key,
            app_key_fingerprint: credential.secret_fingerprint,
        })
    }

    pub fn create_open_app_with_secret_reveal(
        &self,
        command: CreateOpenAppCommand,
    ) -> Result<ManagedOpenAppSecretRevealResult, OpenAppError> {
        let created = self.create_open_app(command)?;
        self.build_secret_reveal_result(created)
    }

    pub fn update_open_app(
        &self,
        open_app_code: &str,
        command: UpdateOpenAppCommand,
    ) -> Result<ManagedOpenAppDetail, OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let name = required_trimmed(command.name, OpenAppError::NameRequired)?;
        let description = optional_trimmed(command.description);
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(mut open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };

        open_app.name = name;
        open_app.description = description;
        open_app.status = command.status;
        open_app.updated_at = current_timestamp();
        repository.update_open_app(&open_app)?;

        let auth_state = repository
            .load_auth_state(&open_app_code)?
            .ok_or(OpenAppError::NotFound)?;
        Ok(map_detail_from_auth_state(&auth_state))
    }

    pub fn rotate_open_app_key(&self, open_app_code: &str) -> Result<CreatedOpenApp, OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let now = current_timestamp();
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(mut open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };
        if open_app.status != OpenAppStatus::Enabled {
            return Err(OpenAppError::OpenAppDisabled);
        }

        let mut next_id = repository.next_id()?;
        let app_key = random_base64url(32);
        let credential = build_active_credential(
            &mut next_id,
            &self.secret_box,
            &app_key,
            &now,
            &open_app.open_app_code,
        )?;
        repository.rotate_credential(&open_app.open_app_code, &now, &credential)?;
        open_app.updated_at = now.clone();
        open_app.last_rotated_at = Some(now);
        repository.update_open_app(&open_app)?;
        Ok(CreatedOpenApp {
            open_app,
            app_key,
            app_key_fingerprint: credential.secret_fingerprint,
        })
    }

    pub fn rotate_open_app_key_with_secret_reveal(
        &self,
        open_app_code: &str,
    ) -> Result<ManagedOpenAppSecretRevealResult, OpenAppError> {
        let rotated = self.rotate_open_app_key(open_app_code)?;
        self.build_secret_reveal_result(rotated)
    }

    pub fn disable_open_app(&self, open_app_code: &str) -> Result<OpenApp, OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(mut open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };
        open_app.status = OpenAppStatus::Disabled;
        open_app.updated_at = current_timestamp();
        repository.update_open_app(&open_app)?;
        Ok(open_app)
    }

    pub fn replace_user_grants(
        &self,
        open_app_code: &str,
        command: ReplaceUserGrantsCommand,
    ) -> Result<ManagedOpenAppDetail, OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let user_codes = normalize_user_codes(command.user_codes)?;
        for user_code in &user_codes {
            self.ensure_grantable_user(user_code)?;
        }

        let now = current_timestamp();
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };

        let mut next_id = repository.next_id()?;
        let grants = user_codes
            .into_iter()
            .map(|user_code| {
                let grant = OpenAppUserGrant {
                    id: SnowflakeId::new(next_id),
                    grant_code: format!("OPENAPP-USER-GRANT-{next_id}"),
                    open_app_code: open_app.open_app_code.clone(),
                    user_code,
                    created_at: now.clone(),
                };
                next_id += 1;
                grant
            })
            .collect::<Vec<_>>();
        repository.replace_user_grants(&open_app.open_app_code, &grants)?;

        let auth_state = repository
            .load_auth_state(&open_app_code)?
            .ok_or(OpenAppError::NotFound)?;
        Ok(map_detail_from_auth_state(&auth_state))
    }

    pub fn replace_capability_grants(
        &self,
        open_app_code: &str,
        command: ReplaceCapabilityGrantsCommand,
    ) -> Result<ManagedOpenAppDetail, OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let capability_grants = normalize_capability_grants(command.grants)?;
        for grant in &capability_grants {
            validate_open_api_grant(&grant.capability_code, &grant.operation_code)?;
        }

        let now = current_timestamp();
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };

        let mut next_id = repository.next_id()?;
        let grants = capability_grants
            .into_iter()
            .map(|grant| {
                let capability_grant = OpenAppCapabilityGrant {
                    id: SnowflakeId::new(next_id),
                    grant_code: format!("OPENAPP-CAP-GRANT-{next_id}"),
                    open_app_code: open_app.open_app_code.clone(),
                    capability_code: grant.capability_code,
                    operation_code: grant.operation_code,
                    created_at: now.clone(),
                };
                next_id += 1;
                capability_grant
            })
            .collect::<Vec<_>>();
        repository.replace_capability_grants(&open_app.open_app_code, &grants)?;

        let auth_state = repository
            .load_auth_state(&open_app_code)?
            .ok_or(OpenAppError::NotFound)?;
        Ok(map_detail_from_auth_state(&auth_state))
    }

    pub fn list_capability_catalog(&self) -> Vec<OpenApiCapabilityCatalogGroup> {
        list_open_api_capability_catalog()
    }

    pub fn grant_user(&self, open_app_code: &str, user_code: &str) -> Result<(), OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let user_code = required_trimmed(user_code.to_owned(), OpenAppError::InvalidUserCode)?;
        self.ensure_grantable_user(&user_code)?;
        let now = current_timestamp();
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };
        let next_id = repository.next_id()?;
        let grant = OpenAppUserGrant {
            id: SnowflakeId::new(next_id),
            grant_code: format!("OPENAPP-USER-GRANT-{next_id}"),
            open_app_code: open_app.open_app_code,
            user_code,
            created_at: now,
        };
        repository.insert_user_grant(&grant)?;
        Ok(())
    }

    pub fn grant_capability(
        &self,
        open_app_code: &str,
        capability_code: &str,
        operation_code: &str,
    ) -> Result<(), OpenAppError> {
        let open_app_code =
            required_trimmed(open_app_code.to_owned(), OpenAppError::InvalidOpenAppCode)?;
        let capability_code =
            required_trimmed(capability_code.to_owned(), OpenAppError::InvalidCapability)?;
        let operation_code =
            required_trimmed(operation_code.to_owned(), OpenAppError::InvalidOperation)?;
        validate_open_api_grant(&capability_code, &operation_code)?;

        let now = current_timestamp();
        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        let Some(open_app) = repository.find_open_app_by_open_app_code(&open_app_code)? else {
            return Err(OpenAppError::NotFound);
        };
        let next_id = repository.next_id()?;
        let grant = OpenAppCapabilityGrant {
            id: SnowflakeId::new(next_id),
            grant_code: format!("OPENAPP-CAP-GRANT-{next_id}"),
            open_app_code: open_app.open_app_code,
            capability_code,
            operation_code,
            created_at: now,
        };
        repository.insert_capability_grant(&grant)?;
        Ok(())
    }

    fn build_secret_reveal_result(
        &self,
        created: CreatedOpenApp,
    ) -> Result<ManagedOpenAppSecretRevealResult, OpenAppError> {
        let detail = self.get_open_app_detail(&created.open_app.open_app_code)?;
        Ok(ManagedOpenAppSecretRevealResult {
            open_app: detail,
            secret_reveal: ManagedOpenAppSecretReveal {
                open_app_code: created.open_app.open_app_code,
                app_key: created.app_key,
                app_key_fingerprint: created.app_key_fingerprint,
            },
        })
    }
}
