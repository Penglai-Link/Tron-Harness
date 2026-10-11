use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::skill::SkillError, response::ApiResponse};

pub fn skill_error_response(error: SkillError, locale: &str) -> Response {
    let descriptor = error.descriptor();
    let (status, _legacy_code, message) = match error {
        SkillError::SearchQueryRequired => (
            StatusCode::BAD_REQUEST,
            "SKILL_SEARCH_QUERY_REQUIRED",
            "search query is required",
        ),
        SkillError::SourceRefRequired => (
            StatusCode::BAD_REQUEST,
            "SKILL_SOURCE_REQUIRED",
            "sourceRef is required",
        ),
        SkillError::SkillNameRequired => (
            StatusCode::BAD_REQUEST,
            "SKILL_NAME_REQUIRED",
            "skillName is required",
        ),
        SkillError::InvalidAgent => (
            StatusCode::BAD_REQUEST,
            "SKILL_AGENT_INVALID",
            "internal agent was not found",
        ),
        SkillError::Unavailable => (
            StatusCode::BAD_REQUEST,
            "SKILL_UNAVAILABLE",
            "skill is unavailable",
        ),
        SkillError::NotFound => (
            StatusCode::NOT_FOUND,
            "SKILL_NOT_FOUND",
            "skill was not found",
        ),
        SkillError::DuplicateSkill => (
            StatusCode::CONFLICT,
            "SKILL_NAME_CONFLICT",
            "skill name conflicts with an installed skill",
        ),
        SkillError::InvalidRegistryPath => (
            StatusCode::BAD_REQUEST,
            "SKILL_REGISTRY_PATH_INVALID",
            "invalid skill registry path",
        ),
        SkillError::StorageFailure => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SKILL_STORAGE_FAILED",
            "skill storage operation failed",
        ),
        SkillError::CommandFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SKILL_COMMAND_FAILED",
            "skills command execution failed",
        ),
    };

    let translated = match descriptor.message_key {
        "tron.skill.searchqueryrequired" => "搜索内容不能为空",
        "tron.skill.sourcerefrequired" => "技能来源不能为空",
        "tron.skill.skillnamerequired" => "技能名称不能为空",
        "tron.skill.invalidagent" => "内部 Agent 不存在",
        "tron.skill.unavailable" => "技能不可分配：所属能力包已禁用或资源不存在",
        "tron.skill.notfound" => "技能不存在",
        "tron.skill.duplicateskill" => "技能名称与已安装技能冲突",
        "tron.skill.invalidregistrypath" => "技能目录无效",
        "tron.skill.storagefailure" => "技能存储操作失败",
        "tron.skill.commandfailed" => "技能安装命令执行失败",
        _ => "技能操作失败",
    };
    (
        status,
        Json(ApiResponse::protocol_error(
            descriptor.code,
            if locale.starts_with("en") {
                message
            } else {
                translated
            },
        )),
    )
        .into_response()
}
