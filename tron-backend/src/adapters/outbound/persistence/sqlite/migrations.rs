pub(super) const USERS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0001_create_user_management_users.sql");
pub(super) const ASSIGNMENTS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0002_create_user_management_assignments.sql");
pub(super) const PASSWORD_RESET_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0003_create_user_password_reset_events.sql");
pub(super) const AUTH_REFRESH_SESSIONS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0004_create_auth_refresh_sessions.sql");
pub(super) const AUTH_OAUTH_IDENTITIES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0005_create_user_oauth_identities.sql");
pub(super) const ROLES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0004_create_role_management_roles.sql");
pub(super) const ROLE_PERMISSIONS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0005_create_role_management_permissions.sql");
