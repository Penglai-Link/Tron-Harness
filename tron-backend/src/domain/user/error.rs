#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserError {
    NotFound,
    InvalidUserCode,
    DuplicateUserCode,
    DuplicateUsername,
    DuplicateEmail,
    RequiredProfileField,
    WeakInitialPassword,
    InvalidStatus,
    InvalidRoleCode,
    InvalidSpaceCode,
    PasswordHashFailed,
    StorageFailure,
}
