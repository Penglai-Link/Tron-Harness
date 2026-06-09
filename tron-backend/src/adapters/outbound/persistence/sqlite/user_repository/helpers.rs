use super::*;

impl SqliteUserRepository {
    pub(super) fn apply_migrations(&mut self) -> Result<(), UserError> {
        self.connection
            .execute_batch(USERS_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(ASSIGNMENTS_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(PASSWORD_RESET_MIGRATION)
            .map_err(storage_error)?;
        self.connection
            .execute_batch(AUTH_REFRESH_SESSIONS_MIGRATION)
            .map_err(storage_error)?;
        if !self.refresh_session_schema_supports_ldap()? {
            self.connection
                .execute_batch(AUTH_REFRESH_SESSIONS_LDAP_MIGRATION)
                .map_err(storage_error)?;
        }
        self.connection
            .execute_batch(AUTH_OAUTH_IDENTITIES_MIGRATION)
            .map_err(storage_error)
    }

    fn refresh_session_schema_supports_ldap(&self) -> Result<bool, UserError> {
        let create_sql = self
            .connection
            .query_row(
                "SELECT sql
                 FROM sqlite_master
                 WHERE type = 'table' AND name = 'auth_refresh_sessions'
                 LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        Ok(create_sql
            .as_deref()
            .is_some_and(|sql| sql.contains("'ldap'")))
    }

    pub(super) fn all_users(&mut self) -> Result<Vec<User>, UserError> {
        let user_codes = {
            let mut statement = self
                .connection
                .prepare("SELECT user_code FROM users ORDER BY display_name ASC")
                .map_err(storage_error)?;
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?
        };

        user_codes
            .iter()
            .map(|user_code| self.get_user(user_code))
            .collect()
    }

    pub(super) fn role_codes(&self, user_id: i64) -> Result<Vec<String>, UserError> {
        self.assignment_codes(
            "SELECT role_code FROM user_role_assignments WHERE user_id = ?1 ORDER BY id ASC",
            user_id,
        )
    }

    pub(super) fn space_codes(&self, user_id: i64) -> Result<Vec<String>, UserError> {
        self.assignment_codes(
            "SELECT space_code FROM user_space_assignments WHERE user_id = ?1 ORDER BY id ASC",
            user_id,
        )
    }

    pub(super) fn assignment_codes(
        &self,
        sql: &str,
        user_id: i64,
    ) -> Result<Vec<String>, UserError> {
        let mut statement = self.connection.prepare(sql).map_err(storage_error)?;
        statement
            .query_map([user_id], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    pub(super) fn insert_assignments(
        &mut self,
        table: &str,
        code_column: &str,
        user: &User,
        codes: &[String],
    ) -> Result<(), UserError> {
        for code in codes {
            let id = self.next_id()?;
            let sql = format!(
                "INSERT INTO {table} (id, user_id, user_code, {code_column}, assigned_at) VALUES (?1, ?2, ?3, ?4, ?5)"
            );
            self.connection
                .execute(
                    &sql,
                    params![
                        id,
                        user.id.value(),
                        user.user_code,
                        code,
                        timestamp_to_millis(&user.updated_at),
                    ],
                )
                .map_err(storage_error)?;
        }
        Ok(())
    }

    pub(super) fn scalar_exists(
        &self,
        sql: &str,
        value: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        let count: i64 = self
            .connection
            .query_row(sql, params![value, except_user_code], |row| row.get(0))
            .map_err(storage_error)?;
        Ok(count > 0)
    }

    pub(super) fn count(&self, sql: &str) -> Result<usize, UserError> {
        let count = self
            .connection
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .map_err(storage_error)?;
        usize::try_from(count).map_err(|_| UserError::StorageFailure)
    }

    pub(super) fn user_by_code_for_auth(
        &mut self,
        user_code: &str,
    ) -> Result<Option<User>, AuthError> {
        self.get_user(user_code)
            .map(Some)
            .or_else(|error| match error {
                UserError::NotFound => Ok(None),
                _ => Err(AuthError::StorageFailure),
            })
    }

    pub(super) fn query_user_code(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Option<String>, AuthError> {
        self.connection
            .query_row(sql, params, |row| row.get::<_, String>(0))
            .optional()
            .map_err(|_| AuthError::StorageFailure)
    }

    pub(super) fn source_type_for_user_code(
        &self,
        user_code: &str,
    ) -> Result<crate::domain::user::UserSourceType, UserError> {
        let provider_code = self
            .connection
            .query_row(
                "SELECT provider_code
                 FROM user_oauth_identities
                 WHERE user_code = ?1
                 ORDER BY linked_at ASC
                 LIMIT 1",
                [user_code],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        Ok(user_source_type_from_provider_code(provider_code))
    }

    pub(super) fn normalize_legacy_oauth_profile(&self, mut user: User) -> User {
        let single_token_display_name = user.display_name.split_whitespace().count() <= 1;
        let provider_fallback_last_name = user.last_name.eq_ignore_ascii_case("github");

        if user.source_type == UserSourceType::GitHub
            && single_token_display_name
            && provider_fallback_last_name
        {
            let normalized_last_name = if user.first_name.trim().is_empty() {
                user.display_name.trim()
            } else {
                user.first_name.trim()
            };

            if !normalized_last_name.is_empty() {
                user.last_name = normalized_last_name.to_owned();
            }
        }

        user
    }
}
