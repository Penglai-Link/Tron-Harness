use super::*;

impl UserRepository for SqliteUserRepository {
    fn list_users(&mut self, query: ListUsersQuery) -> Result<PagedUsers, UserError> {
        let current = query.current.or(query.page).unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let search = query.search.map(|value| value.to_ascii_lowercase());

        let mut records = self
            .all_users()?
            .into_iter()
            .filter(|user| {
                if let Some(status) = query.status {
                    user.status == status
                } else if query.include_archived.unwrap_or(false) {
                    true
                } else {
                    user.status == UserStatus::Active
                }
            })
            .filter(|user| {
                query
                    .role_code
                    .as_ref()
                    .is_none_or(|role| user.role_codes.iter().any(|value| value == role))
            })
            .filter(|user| {
                query
                    .space_code
                    .as_ref()
                    .is_none_or(|space| user.space_codes.iter().any(|value| value == space))
            })
            .filter(|user| {
                search.as_ref().is_none_or(|search| {
                    user.user_code.to_ascii_lowercase().contains(search)
                        || user.username.to_ascii_lowercase().contains(search)
                        || user.display_name.to_ascii_lowercase().contains(search)
                        || user.email.to_ascii_lowercase().contains(search)
                })
            })
            .collect::<Vec<_>>();

        records.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        let total = records.len();
        let start = ((current - 1) * size).min(total);
        let end = (start + size).min(total);

        Ok(PagedUsers {
            current,
            size,
            total,
            records: records[start..end].to_vec(),
        })
    }

    fn get_user(&mut self, user_code: &str) -> Result<User, UserError> {
        let source_type = self.source_type_for_user_code(user_code)?;
        let row = self
            .connection
            .query_row(
                "SELECT id, user_code, username, first_name, last_name, display_name, email, phone_country_code, phone_number, avatar_url, locale, timezone, status, password_hash, password_reset_required, last_active_at, created_at, updated_at, archived_at FROM users WHERE user_code = ?1",
                [user_code],
                |row| {
                    Ok(StoredUserRow {
                        id: row.get(0)?,
                        user_code: row.get(1)?,
                        source_type,
                        username: row.get(2)?,
                        first_name: row.get(3)?,
                        last_name: row.get(4)?,
                        display_name: row.get(5)?,
                        email: row.get(6)?,
                        phone_country_code: row.get(7)?,
                        phone_number: row.get(8)?,
                        avatar_url: row.get(9)?,
                        locale: row.get(10)?,
                        timezone: row.get(11)?,
                        status: row.get(12)?,
                        password_hash: row.get(13)?,
                        password_reset_required: row.get(14)?,
                        last_active_at: row.get(15)?,
                        created_at: row.get(16)?,
                        updated_at: row.get(17)?,
                        archived_at: row.get(18)?,
                    })
                },
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(UserError::NotFound)?;

        let role_codes = self.role_codes(row.id)?;
        let space_codes = self.space_codes(row.id)?;

        Ok(self.normalize_legacy_oauth_profile(User {
            id: SnowflakeId::new(row.id),
            user_code: row.user_code,
            source_type: row.source_type,
            username: row.username,
            first_name: row.first_name,
            last_name: row.last_name,
            display_name: row.display_name,
            email: row.email,
            phone: phone_from_parts(row.phone_country_code, row.phone_number),
            avatar_url: row.avatar_url,
            locale: row.locale.unwrap_or_else(|| "en-US".into()),
            timezone: row.timezone.unwrap_or_else(|| "America/Los_Angeles".into()),
            status: user_status_from_storage(&row.status)?,
            role_codes,
            space_codes,
            password_hash: row.password_hash,
            password_reset_required: row.password_reset_required != 0,
            last_active_at: row.last_active_at.map(timestamp_from_millis),
            created_at: timestamp_from_millis(row.created_at),
            updated_at: timestamp_from_millis(row.updated_at),
            archived_at: row.archived_at.map(timestamp_from_millis),
        }))
    }

    fn statistics(&mut self) -> Result<UserStatistics, UserError> {
        Ok(UserStatistics {
            total_users: self.count("SELECT COUNT(*) FROM users")?,
            active_users: self.count("SELECT COUNT(*) FROM users WHERE status = 'active'")?,
            role_assignments: self.count("SELECT COUNT(*) FROM user_role_assignments")?,
            space_assignments: self.count("SELECT COUNT(*) FROM user_space_assignments")?,
        })
    }

    fn next_id(&mut self) -> Result<i64, UserError> {
        let max_id: i64 = self
            .connection
            .query_row(
                "SELECT max(id) FROM (
                    SELECT id FROM users
                    UNION ALL SELECT id FROM user_role_assignments
                    UNION ALL SELECT id FROM user_space_assignments
                    UNION ALL SELECT id FROM user_password_reset_events
                    UNION ALL SELECT id FROM auth_refresh_sessions
                    UNION ALL SELECT id FROM user_oauth_identities
                )",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(734_200_000_000_000_000);
        Ok(max_id + 1)
    }

    fn next_user_code(&mut self, username: &str) -> Result<String, UserError> {
        let mut code = format!("USR-{}", normalize_code_segment(username));
        if code == "USR-" {
            code = "USR-NEW-USER".into();
        }
        if !self.user_code_exists(&code, None)? {
            return Ok(code);
        }

        let mut suffix = 2;
        loop {
            let candidate = format!("{code}-{suffix}");
            if !self.user_code_exists(&candidate, None)? {
                return Ok(candidate);
            }
            suffix += 1;
        }
    }

    fn user_code_exists(
        &mut self,
        user_code: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        self.scalar_exists(
            "SELECT COUNT(*) FROM users WHERE user_code = ?1 AND (?2 IS NULL OR user_code != ?2)",
            user_code,
            except_user_code,
        )
    }

    fn username_exists(
        &mut self,
        username: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        self.scalar_exists(
            "SELECT COUNT(*) FROM users WHERE username = ?1 COLLATE NOCASE AND (?2 IS NULL OR user_code != ?2)",
            username,
            except_user_code,
        )
    }

    fn email_exists(
        &mut self,
        email: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError> {
        self.scalar_exists(
            "SELECT COUNT(*) FROM users WHERE email = lower(?1) COLLATE NOCASE AND (?2 IS NULL OR user_code != ?2)",
            email,
            except_user_code,
        )
    }

    fn insert_user(&mut self, user: &User) -> Result<(), UserError> {
        self.connection
            .execute(
                "INSERT INTO users (
                    id, user_code, username, first_name, last_name, display_name, email,
                    phone_country_code, phone_number, avatar_url, locale, timezone, status,
                    password_hash, password_hash_algorithm, password_updated_at,
                    password_reset_required, last_active_at, archived_at, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, lower(?7), ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'bcrypt', ?15, ?16, ?17, ?18, ?19, ?20)",
                params![
                    user.id.value(),
                    user.user_code,
                    user.username,
                    user.first_name,
                    user.last_name,
                    user.display_name,
                    user.email,
                    user.phone.as_ref().map(|phone| phone.country_code.as_str()),
                    user.phone.as_ref().map(|phone| phone.number.as_str()),
                    user.avatar_url,
                    user.locale,
                    user.timezone,
                    user_status_to_storage(user.status),
                    user.password_hash,
                    timestamp_to_millis(&user.updated_at),
                    bool_to_int(user.password_reset_required),
                    user.last_active_at.as_deref().map(timestamp_to_millis),
                    user.archived_at.as_deref().map(timestamp_to_millis),
                    timestamp_to_millis(&user.created_at),
                    timestamp_to_millis(&user.updated_at),
                ],
            )
            .map_err(storage_error)?;
        self.insert_assignments("user_role_assignments", "role_code", user, &user.role_codes)?;
        self.insert_assignments(
            "user_space_assignments",
            "space_code",
            user,
            &user.space_codes,
        )
    }

    fn update_user(&mut self, user: &User) -> Result<(), UserError> {
        self.connection
            .execute(
                "UPDATE users SET
                    username = ?2,
                    first_name = ?3,
                    last_name = ?4,
                    display_name = ?5,
                    email = lower(?6),
                    phone_country_code = ?7,
                    phone_number = ?8,
                    avatar_url = ?9,
                    locale = ?10,
                    timezone = ?11,
                    status = ?12,
                    password_hash = ?13,
                    password_updated_at = ?14,
                    password_reset_required = ?15,
                    last_active_at = ?16,
                    archived_at = ?17,
                    updated_at = ?18,
                    version = version + 1
                WHERE user_code = ?1",
                params![
                    user.user_code,
                    user.username,
                    user.first_name,
                    user.last_name,
                    user.display_name,
                    user.email,
                    user.phone.as_ref().map(|phone| phone.country_code.as_str()),
                    user.phone.as_ref().map(|phone| phone.number.as_str()),
                    user.avatar_url,
                    user.locale,
                    user.timezone,
                    user_status_to_storage(user.status),
                    user.password_hash,
                    timestamp_to_millis(&user.updated_at),
                    bool_to_int(user.password_reset_required),
                    user.last_active_at.as_deref().map(timestamp_to_millis),
                    user.archived_at.as_deref().map(timestamp_to_millis),
                    timestamp_to_millis(&user.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn replace_role_codes(&mut self, user: &User) -> Result<(), UserError> {
        self.update_user(user)?;
        self.connection
            .execute(
                "DELETE FROM user_role_assignments WHERE user_id = ?1",
                [user.id.value()],
            )
            .map_err(storage_error)?;
        self.insert_assignments("user_role_assignments", "role_code", user, &user.role_codes)
    }

    fn replace_space_codes(&mut self, user: &User) -> Result<(), UserError> {
        self.update_user(user)?;
        self.connection
            .execute(
                "DELETE FROM user_space_assignments WHERE user_id = ?1",
                [user.id.value()],
            )
            .map_err(storage_error)?;
        self.insert_assignments(
            "user_space_assignments",
            "space_code",
            user,
            &user.space_codes,
        )
    }

    fn record_password_reset(&mut self, user: &User, reset_id: i64) -> Result<(), UserError> {
        self.update_user(user)?;
        self.connection
            .execute(
                "INSERT INTO user_password_reset_events (
                    id, reset_code, user_id, user_code, password_hash_algorithm,
                    password_reset_required_after, temporary_password_returned, created_at
                ) VALUES (?1, ?2, ?3, ?4, 'bcrypt', 1, 1, ?5)",
                params![
                    reset_id,
                    format!("RESET-{reset_id}"),
                    user.id.value(),
                    user.user_code,
                    timestamp_to_millis(&user.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn find_user_by_login(&mut self, login: &str) -> Result<Option<User>, AuthError> {
        let normalized = login.trim().to_ascii_lowercase();
        let user_code = self.query_user_code(
            "SELECT user_code FROM users WHERE username = ?1 COLLATE NOCASE OR email = ?2 COLLATE NOCASE LIMIT 1",
            params![login.trim(), normalized],
        )?;
        match user_code {
            Some(user_code) => self.user_by_code_for_auth(&user_code),
            None => Ok(None),
        }
    }

    fn find_active_user_by_email(&mut self, email: &str) -> Result<Option<User>, AuthError> {
        let user_code = self.query_user_code(
            "SELECT user_code FROM users WHERE email = lower(?1) COLLATE NOCASE AND status = 'active' LIMIT 1",
            params![email.trim()],
        )?;
        match user_code {
            Some(user_code) => self.user_by_code_for_auth(&user_code),
            None => Ok(None),
        }
    }

    fn find_user_by_provider_subject(
        &mut self,
        provider_code: &str,
        provider_subject: &str,
    ) -> Result<Option<User>, AuthError> {
        let user_code = self.query_user_code(
            "SELECT user_code FROM user_oauth_identities WHERE provider_code = ?1 AND provider_subject = ?2 LIMIT 1",
            params![provider_code, provider_subject],
        )?;
        match user_code {
            Some(user_code) => self.user_by_code_for_auth(&user_code),
            None => Ok(None),
        }
    }

    fn create_refresh_session(&mut self, session: &RefreshSession) -> Result<(), AuthError> {
        self.connection
            .execute(
                "INSERT INTO auth_refresh_sessions (
                    id, session_code, user_id, user_code, auth_method, provider_code,
                    refresh_token_hash, expires_at, created_at, updated_at, last_used_at, revoked_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    session.id,
                    session.session_code,
                    session.user_id,
                    session.user_code,
                    match session.auth_method {
                        AuthMethod::Internal => "internal",
                        AuthMethod::Ldap => "ldap",
                        AuthMethod::Oauth2 => "oauth2",
                    },
                    session.provider_code,
                    session.refresh_token_hash,
                    session.expires_at,
                    session.created_at,
                    session.updated_at,
                    session.last_used_at,
                    session.revoked_at,
                ],
            )
            .map_err(|_| AuthError::StorageFailure)?;
        Ok(())
    }

    fn find_refresh_session(
        &mut self,
        session_code: &str,
    ) -> Result<Option<RefreshSession>, AuthError> {
        self.connection
            .query_row(
                "SELECT id, session_code, user_id, user_code, auth_method, provider_code, refresh_token_hash, expires_at, created_at, updated_at, last_used_at, revoked_at
                 FROM auth_refresh_sessions WHERE session_code = ?1 LIMIT 1",
                [session_code],
                |row| {
                    Ok(RefreshSession {
                        id: row.get(0)?,
                        session_code: row.get(1)?,
                        user_id: row.get(2)?,
                        user_code: row.get(3)?,
                        auth_method: match row.get::<_, String>(4)?.as_str() {
                            "internal" => AuthMethod::Internal,
                            "ldap" => AuthMethod::Ldap,
                            _ => AuthMethod::Oauth2,
                        },
                        provider_code: row.get(5)?,
                        refresh_token_hash: row.get(6)?,
                        expires_at: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                        last_used_at: row.get(10)?,
                        revoked_at: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(|_| AuthError::StorageFailure)
    }

    fn replace_refresh_session(&mut self, session: &RefreshSession) -> Result<(), AuthError> {
        self.connection
            .execute(
                "UPDATE auth_refresh_sessions SET
                    refresh_token_hash = ?2,
                    expires_at = ?3,
                    updated_at = ?4,
                    last_used_at = ?5,
                    revoked_at = ?6
                 WHERE session_code = ?1",
                params![
                    session.session_code,
                    session.refresh_token_hash,
                    session.expires_at,
                    session.updated_at,
                    session.last_used_at,
                    session.revoked_at,
                ],
            )
            .map_err(|_| AuthError::StorageFailure)?;
        Ok(())
    }

    fn revoke_refresh_session(
        &mut self,
        session_code: &str,
        revoked_at: i64,
    ) -> Result<(), AuthError> {
        self.connection
            .execute(
                "UPDATE auth_refresh_sessions
                 SET revoked_at = ?2, updated_at = ?2
                 WHERE session_code = ?1",
                params![session_code, revoked_at],
            )
            .map_err(|_| AuthError::StorageFailure)?;
        Ok(())
    }

    fn upsert_oauth_identity(&mut self, identity: &OAuth2IdentityLink) -> Result<(), AuthError> {
        self.connection
            .execute(
                "INSERT INTO user_oauth_identities (
                    id, user_id, user_code, provider_code, provider_subject, email, display_name, linked_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, lower(?6), ?7, ?8, ?9)
                ON CONFLICT(provider_code, provider_subject) DO UPDATE SET
                    user_id = excluded.user_id,
                    user_code = excluded.user_code,
                    email = excluded.email,
                    display_name = excluded.display_name,
                    updated_at = excluded.updated_at",
                params![
                    identity.id,
                    identity.user_id,
                    identity.user_code,
                    identity.provider_code,
                    identity.provider_subject,
                    identity.email,
                    identity.display_name,
                    identity.linked_at,
                    identity.updated_at,
                ],
            )
            .map_err(|_| AuthError::StorageFailure)?;
        Ok(())
    }
}
