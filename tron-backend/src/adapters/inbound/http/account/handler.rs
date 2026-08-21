use axum::{
    Json,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Multipart, State, multipart::MultipartRejection},
    http::{
        HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use tempfile::tempfile;
use tokio::io::AsyncReadExt;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::{
        resource::{ResourceService, SYSTEM_AVATAR_REFERENCE},
        user::{AccountError, UpdateAccountProfileCommand, UserService},
    },
    domain::resource::ResourceError,
    response::ApiResponse,
};

use super::{
    error_mapper::{account_error_response, avatar_error_response},
    request::{ChangePasswordRequest, UpdateAccountProfileRequest},
    response::{AccountProfileDto, PasswordChangeDto},
};

pub const MAX_ACCOUNT_AVATAR_BYTES: usize = 2 * 1024 * 1024;

pub fn account_avatar_body_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(MAX_ACCOUNT_AVATAR_BYTES + 1024 * 1024)
}

pub async fn get_profile(State(users): State<UserService>, auth: RequestAuthContext) -> Response {
    match users.get_user(&auth.user().user_code) {
        Ok(user) => Json(ApiResponse::success(AccountProfileDto::new(
            user,
            auth.session.auth_method,
        )))
        .into_response(),
        Err(error) => account_error_response(AccountError::from(error)),
    }
}

pub async fn update_profile(
    State(users): State<UserService>,
    auth: RequestAuthContext,
    Json(request): Json<UpdateAccountProfileRequest>,
) -> Response {
    match users.update_account_profile(auth.user(), request.into()) {
        Ok(user) => Json(ApiResponse::success(AccountProfileDto::new(
            user,
            auth.session.auth_method,
        )))
        .into_response(),
        Err(error) => account_error_response(error),
    }
}

pub async fn change_password(
    State(users): State<UserService>,
    auth: RequestAuthContext,
    Json(request): Json<ChangePasswordRequest>,
) -> Response {
    match users.change_account_password(auth.user(), auth.session.auth_method, request.into()) {
        Ok(result) => Json(ApiResponse::success(PasswordChangeDto::from(result))).into_response(),
        Err(error) => account_error_response(error),
    }
}

pub async fn upload_avatar(
    State(users): State<UserService>,
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Multipart, MultipartRejection>,
) -> Response {
    let multipart = match request {
        Ok(multipart) => multipart,
        Err(error) => {
            return avatar_error_response(
                error.status(),
                "ACCOUNT_AVATAR_INVALID",
                "avatar upload request is invalid",
            );
        }
    };
    let (file_name, file) = match parse_avatar_upload(multipart).await {
        Ok(value) => value,
        Err(AvatarUploadError::TooLarge) => {
            return avatar_error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "ACCOUNT_AVATAR_TOO_LARGE",
                "avatar file must not exceed 2 MB",
            );
        }
        Err(AvatarUploadError::Invalid) => {
            return avatar_error_response(
                StatusCode::BAD_REQUEST,
                "ACCOUNT_AVATAR_INVALID",
                "avatar must be a PNG, JPEG, WEBP, or GIF image",
            );
        }
    };
    let actor_user_code = auth.user().user_code.clone();
    let service = resources.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut file = file;
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        service.replace_system_avatar(&actor_user_code, &file_name, &mut file)
    })
    .await;
    match result {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            return crate::adapters::inbound::http::resources::error_mapper::resource_error_response(
                error,
                &auth.locale,
            );
        }
        Err(_) => {
            return avatar_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ACCOUNT_AVATAR_STORAGE_FAILED",
                "avatar storage operation failed",
            );
        }
    }

    match users.update_account_profile(
        auth.user(),
        UpdateAccountProfileCommand {
            first_name: None,
            last_name: None,
            display_name: None,
            phone: None,
            avatar_url: Some(Some(SYSTEM_AVATAR_REFERENCE.to_string())),
            locale: None,
            timezone: None,
        },
    ) {
        Ok(user) => Json(ApiResponse::success(AccountProfileDto::new(
            user,
            auth.session.auth_method,
        )))
        .into_response(),
        Err(error) => account_error_response(error),
    }
}

pub async fn download_avatar(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
) -> Response {
    let actor_user_code = auth.user().user_code.clone();
    let (download, record) = match resources.download_system_avatar(&actor_user_code) {
        Ok(value) => value,
        Err(ResourceError::NotFound) => {
            return avatar_error_response(
                StatusCode::NOT_FOUND,
                "ACCOUNT_AVATAR_NOT_FOUND",
                "account avatar was not found",
            );
        }
        Err(error) => {
            return crate::adapters::inbound::http::resources::error_mapper::resource_error_response(
                error,
                &auth.locale,
            );
        }
    };
    let service = resources.clone();
    let file = match tempfile() {
        Ok(file) => file,
        Err(_) => {
            return avatar_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ACCOUNT_AVATAR_STORAGE_FAILED",
                "avatar storage operation failed",
            );
        }
    };
    let result = tokio::task::spawn_blocking(move || {
        let mut file = file;
        service.download_binary_storage(&record, &mut file)?;
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        Ok::<_, ResourceError>(file)
    })
    .await;
    let file = match result {
        Ok(Ok(file)) => file,
        Ok(Err(error)) => {
            return crate::adapters::inbound::http::resources::error_mapper::resource_error_response(
                error,
                &auth.locale,
            );
        }
        Err(_) => {
            return avatar_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ACCOUNT_AVATAR_STORAGE_FAILED",
                "avatar storage operation failed",
            );
        }
    };
    let file = tokio::fs::File::from_std(file);
    let content_length = download.byte_size;
    let file_name = download.file_name;
    let content_type = avatar_content_type(&file_name);
    let stream = async_stream::stream! {
        let mut file = file;
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = match file.read(&mut buffer).await {
                Ok(read) => read,
                Err(error) => {
                    yield Err::<Bytes, std::io::Error>(error);
                    break;
                }
            };
            if read == 0 { break; }
            yield Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(&buffer[..read]));
        }
    };
    let mut response = Response::new(Body::from_stream(stream));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    if let Ok(value) = HeaderValue::from_str(&content_length.to_string()) {
        response.headers_mut().insert(CONTENT_LENGTH, value);
    }
    if let Ok(value) = HeaderValue::from_str(&format!(
        "attachment; filename=\"{}\"",
        file_name.replace('"', "")
    )) {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }
    response
}

enum AvatarUploadError {
    Invalid,
    TooLarge,
}

async fn parse_avatar_upload(
    mut multipart: Multipart,
) -> Result<(String, std::fs::File), AvatarUploadError> {
    let mut upload: Option<(String, std::fs::File)> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AvatarUploadError::Invalid)?
    {
        if field.name() != Some("file") || upload.is_some() {
            return Err(AvatarUploadError::Invalid);
        }
        let content_type = field.content_type().unwrap_or_default();
        if !matches!(
            content_type,
            "image/png" | "image/jpeg" | "image/webp" | "image/gif"
        ) {
            return Err(AvatarUploadError::Invalid);
        }
        let file_name = field
            .file_name()
            .map(str::to_owned)
            .filter(|name| !name.trim().is_empty())
            .ok_or(AvatarUploadError::Invalid)?;
        let bytes = field
            .bytes()
            .await
            .map_err(|_| AvatarUploadError::Invalid)?;
        if bytes.is_empty() {
            return Err(AvatarUploadError::Invalid);
        }
        if bytes.len() > MAX_ACCOUNT_AVATAR_BYTES {
            return Err(AvatarUploadError::TooLarge);
        }
        let mut file = tempfile().map_err(|_| AvatarUploadError::Invalid)?;
        std::io::Write::write_all(&mut file, &bytes).map_err(|_| AvatarUploadError::Invalid)?;
        upload = Some((file_name, file));
    }
    upload.ok_or(AvatarUploadError::Invalid)
}

fn avatar_content_type(file_name: &str) -> &'static str {
    match file_name
        .rsplit('.')
        .next()
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}
