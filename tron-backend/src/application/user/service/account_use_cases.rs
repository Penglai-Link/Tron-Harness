use super::*;

impl UserService {
    pub fn update_account_profile(
        &self,
        actor: &AuthenticatedUser,
        command: UpdateAccountProfileCommand,
    ) -> Result<User, AccountError> {
        let user_code = actor.user_code.clone();
        let update = UpdateUserCommand {
            username: None,
            first_name: normalize_required_update(command.first_name)?,
            last_name: normalize_required_update(command.last_name)?,
            display_name: normalize_required_update(command.display_name)?,
            email: None,
            phone: command.phone,
            avatar_url: command
                .avatar_url
                .map(|avatar_url| optional_trimmed(avatar_url)),
            locale: normalize_required_update(command.locale)?,
            timezone: normalize_required_update(command.timezone)?,
            status: None,
        };

        self.patch_user(actor, &user_code, update)
            .map_err(AccountError::from)
    }

    pub fn change_account_password(
        &self,
        actor: &AuthenticatedUser,
        auth_method: AuthMethod,
        command: ChangePasswordCommand,
    ) -> Result<PasswordChangeResult, AccountError> {
        if auth_method != AuthMethod::Internal {
            return Err(AccountError::PasswordChangeUnsupported);
        }

        validate_initial_password(&command.new_password)?;

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(&actor.user_code)?;

        if user.source_type != UserSourceType::Internal {
            return Err(AccountError::PasswordChangeUnsupported);
        }

        let current_password_matches = self
            .password_hasher
            .verify_password(&command.current_password, &user.password_hash)
            .map_err(|_| AccountError::PasswordVerifyFailed)?;
        if !current_password_matches {
            return Err(AccountError::CurrentPasswordIncorrect);
        }

        let password_is_unchanged = self
            .password_hasher
            .verify_password(&command.new_password, &user.password_hash)
            .map_err(|_| AccountError::PasswordVerifyFailed)?;
        if password_is_unchanged {
            return Err(AccountError::PasswordUnchanged);
        }

        user.password_hash = self.password_hasher.hash_password(&command.new_password)?;
        user.password_reset_required = false;
        user.updated_at = current_timestamp();
        repository.update_user(&user)?;

        let updated = user.clone();
        drop(repository);
        self.publish_user_event(
            "account.password_changed",
            "Changed account password",
            actor,
            &updated,
            BusPriority::Critical,
        );

        Ok(PasswordChangeResult {
            changed: true,
            password_reset_required: false,
        })
    }
}

fn normalize_required_update(value: Option<String>) -> Result<Option<String>, AccountError> {
    value
        .map(|value| required_profile_field(Some(value)).map_err(AccountError::from))
        .transpose()
}
