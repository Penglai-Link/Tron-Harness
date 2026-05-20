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
pub(super) const WORKSPACES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260516_0006_create_workspace_management_workspaces.sql"
);
pub(super) const WORKSPACE_ACCESS_GRANTS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260516_0007_create_workspace_management_access_grants.sql"
);
pub(super) const TASKS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260516_0008_create_task_management_tasks.sql");
pub(super) const TASKS_SUPERPOWER_RESET_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260519_0010_reset_task_management_superpower.sql"
);
pub(super) const VARIABLES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260518_0009_create_variable_management_variables.sql"
);
pub(super) const SKILL_REGISTRY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260519_0011_create_skill_registry.sql");
