pub(super) const USERS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0001_create_user_management_users.sql");
pub(super) const ASSIGNMENTS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0002_create_user_management_assignments.sql");
pub(super) const PASSWORD_RESET_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0003_create_user_password_reset_events.sql");
