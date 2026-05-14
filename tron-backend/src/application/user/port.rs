use crate::domain::user::{ListUsersQuery, PagedUsers, User, UserError, UserStatistics};

pub trait UserRepository: Send {
    fn list_users(&mut self, query: ListUsersQuery) -> Result<PagedUsers, UserError>;
    fn get_user(&mut self, user_code: &str) -> Result<User, UserError>;
    fn statistics(&mut self) -> Result<UserStatistics, UserError>;
    fn next_id(&mut self) -> Result<i64, UserError>;
    fn next_user_code(&mut self, username: &str) -> Result<String, UserError>;
    fn user_code_exists(
        &mut self,
        user_code: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError>;
    fn username_exists(
        &mut self,
        username: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError>;
    fn email_exists(
        &mut self,
        email: &str,
        except_user_code: Option<&str>,
    ) -> Result<bool, UserError>;
    fn insert_user(&mut self, user: &User) -> Result<(), UserError>;
    fn update_user(&mut self, user: &User) -> Result<(), UserError>;
    fn replace_role_codes(&mut self, user: &User) -> Result<(), UserError>;
    fn replace_space_codes(&mut self, user: &User) -> Result<(), UserError>;
    fn record_password_reset(&mut self, user: &User, reset_id: i64) -> Result<(), UserError>;
}

pub trait PasswordHasher: Send + Sync {
    fn hash_password(&self, value: &str) -> Result<String, UserError>;
}
