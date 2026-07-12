use serde::Deserialize;

use crate::{
    application::user::{ChangePasswordCommand, UpdateAccountProfileCommand},
    domain::user::PhoneNumber,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountProfileRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub phone: Option<Option<PhoneNumber>>,
    pub avatar_url: Option<Option<String>>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
}

impl From<UpdateAccountProfileRequest> for UpdateAccountProfileCommand {
    fn from(request: UpdateAccountProfileRequest) -> Self {
        Self {
            first_name: request.first_name,
            last_name: request.last_name,
            display_name: request.display_name,
            phone: request.phone,
            avatar_url: request.avatar_url,
            locale: request.locale,
            timezone: request.timezone,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

impl From<ChangePasswordRequest> for ChangePasswordCommand {
    fn from(request: ChangePasswordRequest) -> Self {
        Self {
            current_password: request.current_password,
            new_password: request.new_password,
        }
    }
}
