use serde::Deserialize;

use crate::{
    application::user::{CreateUserCommand, UpdateUserCommand},
    domain::user::PhoneNumber,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
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
    #[serde(default)]
    pub role_codes: Vec<String>,
    #[serde(default)]
    pub space_codes: Vec<String>,
}

impl From<CreateUserRequest> for CreateUserCommand {
    fn from(request: CreateUserRequest) -> Self {
        Self {
            user_code: request.user_code,
            username: request.username,
            first_name: request.first_name,
            last_name: request.last_name,
            display_name: request.display_name,
            email: request.email,
            phone: request.phone,
            avatar_url: request.avatar_url,
            locale: request.locale,
            timezone: request.timezone,
            initial_password: request.initial_password,
            password_reset_required: request.password_reset_required,
            role_codes: request.role_codes,
            space_codes: request.space_codes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
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

impl From<UpdateUserRequest> for UpdateUserCommand {
    fn from(request: UpdateUserRequest) -> Self {
        Self {
            username: request.username,
            first_name: request.first_name,
            last_name: request.last_name,
            display_name: request.display_name,
            email: request.email,
            phone: request.phone,
            avatar_url: request.avatar_url,
            locale: request.locale,
            timezone: request.timezone,
            status: request.status,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceRolesRequest {
    #[serde(default)]
    pub role_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceSpacesRequest {
    #[serde(default)]
    pub space_codes: Vec<String>,
}
