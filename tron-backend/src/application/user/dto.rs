use crate::domain::user::PhoneNumber;

#[derive(Debug, Clone)]
pub struct CreateUserCommand {
    pub user_code: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<PhoneNumber>,
    pub avatar_url: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub initial_password: Option<String>,
    pub password_reset_required: Option<bool>,
    pub role_codes: Vec<String>,
    pub space_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateUserCommand {
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<Option<PhoneNumber>>,
    pub avatar_url: Option<Option<String>>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub status: Option<String>,
}
#[derive(Debug, Clone)]
pub struct StepUpToken {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepUpVariableRevealTarget {
    pub scope: StepUpVariableRevealScope,
    pub workspace_code: Option<String>,
    pub variable_key: String,
}

impl StepUpVariableRevealTarget {
    pub fn platform(variable_key: impl Into<String>) -> Self {
        Self {
            scope: StepUpVariableRevealScope::Platform,
            workspace_code: None,
            variable_key: variable_key.into(),
        }
    }

    pub fn workspace(workspace_code: impl Into<String>, variable_key: impl Into<String>) -> Self {
        Self {
            scope: StepUpVariableRevealScope::Workspace,
            workspace_code: Some(workspace_code.into()),
            variable_key: variable_key.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepUpVariableRevealScope {
    Platform,
    Workspace,
}
