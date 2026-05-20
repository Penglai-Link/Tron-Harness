use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{domain::skill::SkillError, response::ApiResponse};

pub fn skill_error_response(error: SkillError) -> Response {
    let (status, code, message) = match error {
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
        SkillError::WorkspaceRequired => (
            StatusCode::BAD_REQUEST,
            "SKILL_WORKSPACE_REQUIRED",
            "workspaceCode is required",
        ),
        SkillError::InvalidWorkspace => (
            StatusCode::BAD_REQUEST,
            "SKILL_WORKSPACE_INVALID",
            "skill workspace must reference an active workspace",
        ),
        SkillError::NotFound => (
            StatusCode::NOT_FOUND,
            "SKILL_NOT_FOUND",
            "skill was not found",
        ),
        SkillError::DuplicateBinding => (
            StatusCode::CONFLICT,
            "SKILL_BINDING_DUPLICATE",
            "skill binding already exists in the workspace",
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

    (status, Json(ApiResponse::error(code, message))).into_response()
}
