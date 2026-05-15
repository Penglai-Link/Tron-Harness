use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalLoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuth2AuthorizeQuery {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuth2CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}
