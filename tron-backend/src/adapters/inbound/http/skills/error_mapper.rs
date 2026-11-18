use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::skill::SkillError, response::ApiResponse};

pub fn skill_error_response(error: SkillError, locale: &str) -> Response {
    let descriptor = error.descriptor();
    let status = match error {
        SkillError::SearchQueryRequired
        | SkillError::SourceRefRequired
        | SkillError::SkillNameRequired
        | SkillError::InvalidAgent
        | SkillError::Unavailable
        | SkillError::InvalidRegistryPath => StatusCode::BAD_REQUEST,
        SkillError::NotFound => StatusCode::NOT_FOUND,
        SkillError::DuplicateSkill => StatusCode::CONFLICT,
        SkillError::StorageFailure | SkillError::CommandFailed => StatusCode::INTERNAL_SERVER_ERROR,
    };
    // Stable descriptor keys are shared with the application layer; the
    // legacy code from `descriptor.code` stays the API response code.
    let message_key = match descriptor.message_key {
        "tron.skill.searchqueryrequired" => "tron.skill.searchqueryrequired",
        "tron.skill.sourcerefrequired" => "tron.skill.sourcerefrequired",
        "tron.skill.skillnamerequired" => "tron.skill.skillnamerequired",
        "tron.skill.invalidagent" => "tron.skill.invalidagent",
        "tron.skill.unavailable" => "tron.skill.unavailable",
        "tron.skill.notfound" => "tron.skill.notfound",
        "tron.skill.duplicateskill" => "tron.skill.duplicateskill",
        "tron.skill.invalidregistrypath" => "tron.skill.invalidregistrypath",
        "tron.skill.storagefailure" => "tron.skill.storagefailure",
        "tron.skill.commandfailed" => "tron.skill.commandfailed",
        _ => "tron.skill.operation_failed",
    };
    (
        status,
        Json(ApiResponse::protocol_error(
            descriptor.code,
            tron_i18n::t!(message_key, locale = locale),
        )),
    )
        .into_response()
}
