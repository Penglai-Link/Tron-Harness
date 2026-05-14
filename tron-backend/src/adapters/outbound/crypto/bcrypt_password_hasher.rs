use bcrypt::hash;

use crate::{application::user::PasswordHasher, domain::user::UserError};

const BCRYPT_MVP_COST: u32 = 4;

#[derive(Debug, Default, Clone, Copy)]
pub struct BcryptPasswordHasher;

impl PasswordHasher for BcryptPasswordHasher {
    fn hash_password(&self, value: &str) -> Result<String, UserError> {
        hash(value, BCRYPT_MVP_COST).map_err(|_| UserError::PasswordHashFailed)
    }
}
