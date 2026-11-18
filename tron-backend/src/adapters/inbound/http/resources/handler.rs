use std::collections::BTreeSet;

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{
        DefaultBodyLimit, Multipart, State, multipart::MultipartRejection, rejection::JsonRejection,
    },
    http::{
        HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use tempfile::tempfile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::resource::{
        ResourceBinaryDownloadCommand, ResourceCommitBinary, ResourceCreateCommand,
        ResourceDeleteCommand, ResourceService,
    },
    domain::resource::ResourceError,
};

struct ResourceCommitForm {
    command: crate::application::resource::ResourceCreateCommand,
    binary: ResourceCommitBinary,
}

use super::{
    ResourceBinaryActionResponse, ResourceBinaryDownloadRequest, ResourceCommitResponse,
    ResourceCreateRequest, ResourceDeleteRequest, ResourceDeleteResponse, ResourceGetRequest,
    ResourceHttpApiResponse, ResourcePageRequest, ResourcePageResponse, ResourceResponse,
    ResourceUpdateRequest, binary_upload_command,
    error_mapper::{
        internal_error_response, malformed_json_request_response, malformed_request_response,
        resource_error_response, resource_error_with_field,
    },
};

pub async fn page_resources(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourcePageRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.query_resources(&auth.user().user_code, request.into()) {
        Ok(page) => Json(ResourceHttpApiResponse::success(
            ResourcePageResponse::from(page),
        ))
        .into_response(),
        Err(error) => resource_error_response(
            resource_error_with_field(error, "resourceCode", &locale),
            &locale,
        ),
    }
}

pub async fn get_resource(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceGetRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.get_resource(&auth.user().user_code, request.into()) {
        Ok(record) => Json(ResourceHttpApiResponse::success(ResourceResponse::from(
            record,
        )))
        .into_response(),
        Err(error) => resource_error_response(
            resource_error_with_field(error, "resourceCode", &locale),
            &locale,
        ),
    }
}

pub async fn create_resource(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Multipart, MultipartRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let multipart = match request {
        Ok(multipart) => multipart,
        Err(error) => return malformed_request_response(error.status(), &locale),
    };
    let actor_user_code = auth.user().user_code.clone();
    let form: ResourceCommitForm = match parse_create_form(multipart, &resources).await {
        Ok(form) => form.into(),
        Err(UploadParseError::Malformed(status)) => {
            return malformed_request_response(status, &locale);
        }
        Err(UploadParseError::Resource(error)) => {
            return resource_error_response(
                resource_error_with_field(error, "command", &locale),
                &locale,
            );
        }
        Err(UploadParseError::Internal) => return internal_error_response(&locale),
    };
    let service = resources.clone();
    match crate::app::telemetry::spawn_blocking_traced(move || {
        let ResourceCommitForm {
            command,
            mut binary,
        } = form;
        std::io::Seek::seek(&mut binary.source, std::io::SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        service.create_file_resource(&actor_user_code, command, Some(binary))
    })
    .await
    {
        Ok(Ok(result)) => Json(ResourceHttpApiResponse::success(
            ResourceCommitResponse::from(result),
        ))
        .into_response(),
        Ok(Err(error)) => resource_error_response(
            resource_error_with_field(error, "command.resourceCode", &locale),
            &locale,
        ),
        Err(_) => internal_error_response(&locale),
    }
}

pub async fn update_resource(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceUpdateRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.update_resource(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceCommitResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(
            resource_error_with_field(error, "resourceCode", &locale),
            &locale,
        ),
    }
}

pub async fn delete_resource(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceDeleteRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceDeleteCommand = request.into();
    match resources.delete_resource(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceDeleteResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(
            resource_error_with_field(error, "resourceCode", &locale),
            &locale,
        ),
    }
}

pub async fn upload_binary(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Multipart, MultipartRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let multipart = match request {
        Ok(multipart) => multipart,
        Err(error) => return malformed_request_response(error.status(), &locale),
    };
    let actor_user_code = auth.user().user_code.clone();
    let form = match parse_upload_form(
        multipart,
        resources.max_binary_bytes(),
        &resources,
        &actor_user_code,
    )
    .await
    {
        Ok(form) => form,
        Err(UploadParseError::Malformed(status)) => {
            return malformed_request_response(status, &locale);
        }
        Err(UploadParseError::Resource(error)) => {
            return resource_error_response(
                resource_error_with_field(error, "resourceCode", &locale),
                &locale,
            );
        }
        Err(UploadParseError::Internal) => return internal_error_response(&locale),
    };
    let service = resources.clone();
    match crate::app::telemetry::spawn_blocking_traced(move || {
        let mut file = form.file;
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        service.upload_binary_from_reader(
            &actor_user_code,
            form.command,
            &form.file_name,
            &mut file,
        )
    })
    .await
    {
        Ok(Ok(result)) => Json(ResourceHttpApiResponse::success(
            ResourceBinaryActionResponse::from(result),
        ))
        .into_response(),
        Ok(Err(error)) => resource_error_response(
            resource_error_with_field(error, "resourceCode", &locale),
            &locale,
        ),
        Err(_) => internal_error_response(&locale),
    }
}

pub async fn download_binary(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceBinaryDownloadRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceBinaryDownloadCommand = request.into();
    let actor_user_code = auth.user().user_code.clone();
    let service = resources.clone();
    let (download, record) = match resources.download_binary_to_file(&actor_user_code, command) {
        Ok(value) => value,
        Err(error) => {
            return resource_error_response(
                resource_error_with_field(error, "resourceCode", &locale),
                &locale,
            );
        }
    };
    let file = match tempfile() {
        Ok(file) => file,
        Err(_) => return internal_error_response(&locale),
    };
    let download_result = crate::app::telemetry::spawn_blocking_traced(move || {
        let mut file = file;
        service.download_binary_storage(&record, &mut file)?;
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0))
            .map_err(|_| ResourceError::StorageFailure)?;
        Ok::<_, ResourceError>(file)
    })
    .await;
    let file = match download_result {
        Ok(Ok(file)) => file,
        Ok(Err(error)) => return resource_error_response(error, &locale),
        Err(_) => return internal_error_response(&locale),
    };
    let file = tokio::fs::File::from_std(file);
    let content_length = download.byte_size;
    let file_name = download.file_name;
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
            if read == 0 {
                break;
            }
            yield Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(&buffer[..read]));
        }
    };
    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    if let Ok(value) = HeaderValue::from_str(&content_length.to_string()) {
        response.headers_mut().insert(CONTENT_LENGTH, value);
    }
    if let Ok(value) = content_disposition(&file_name) {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }
    response
}

pub(super) fn json_body<T>(
    request: Result<Json<T>, JsonRejection>,
    locale: &str,
) -> Result<T, Response> {
    request
        .map(|Json(value)| value)
        .map_err(|error| malformed_json_request_response(&error, locale))
}

struct UploadBinaryForm {
    command: crate::application::resource::ResourceBinaryUploadCommand,
    file_name: String,
    file: std::fs::File,
}

struct CreateResourceForm {
    command: ResourceCreateCommand,
    binary: ResourceCommitBinary,
}

impl From<CreateResourceForm> for ResourceCommitForm {
    fn from(value: CreateResourceForm) -> Self {
        Self {
            command: value.command,
            binary: value.binary,
        }
    }
}

enum UploadParseError {
    Malformed(StatusCode),
    Resource(ResourceError),
    Internal,
}

const MAX_UPLOAD_CONTROL_FIELD_BYTES: usize = 8 * 1024;

async fn parse_create_form(
    mut multipart: Multipart,
    resources: &ResourceService,
) -> Result<CreateResourceForm, UploadParseError> {
    let mut command = None;
    let mut binary = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| UploadParseError::Malformed(error.status()))?
    {
        let name = field
            .name()
            .map(str::to_owned)
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
        match name.as_str() {
            "command" if command.is_none() && binary.is_none() => {
                let value =
                    read_upload_text_with_limit(field, resources.max_commit_control_bytes())
                        .await?;
                let request: ResourceCreateRequest = serde_json::from_str(&value)
                    .map_err(|_| UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
                let parsed: ResourceCreateCommand =
                    request.try_into().map_err(UploadParseError::Resource)?;
                command = Some(parsed);
            }
            "binary" if command.is_some() && binary.is_none() => {
                let file_name = field
                    .file_name()
                    .map(str::to_owned)
                    .filter(|name| !name.trim().is_empty())
                    .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
                let mut output =
                    tokio::fs::File::from_std(tempfile().map_err(|_| UploadParseError::Internal)?);
                let mut received = 0_u64;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|error| UploadParseError::Malformed(error.status()))?
                {
                    received = received
                        .checked_add(chunk.len() as u64)
                        .ok_or(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE))?;
                    if received > resources.max_binary_bytes() {
                        return Err(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE));
                    }
                    output
                        .write_all(&chunk)
                        .await
                        .map_err(|_| UploadParseError::Internal)?;
                }
                output
                    .flush()
                    .await
                    .map_err(|_| UploadParseError::Internal)?;
                binary = Some(ResourceCommitBinary {
                    source: output.into_std().await,
                    file_name,
                });
            }
            _ => return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST)),
        }
    }
    let command: ResourceCreateCommand =
        command.ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
    let binary = binary.ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
    Ok(CreateResourceForm { command, binary })
}

async fn parse_upload_form(
    mut multipart: Multipart,
    max_binary_bytes: u64,
    resources: &ResourceService,
    actor_user_code: &str,
) -> Result<UploadBinaryForm, UploadParseError> {
    let mut seen = BTreeSet::new();
    let mut workspace_code = None;
    let mut resource_code = None;
    let mut file = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| UploadParseError::Malformed(error.status()))?
    {
        let name = field
            .name()
            .map(str::to_owned)
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
        if !seen.insert(name.clone()) {
            return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST));
        }
        if file.is_some() {
            return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST));
        }
        match name.as_str() {
            "workspaceCode" => workspace_code = Some(read_upload_text(field).await?),
            "resourceCode" => resource_code = Some(read_upload_text(field).await?),
            "file" => {
                let command = binary_upload_command(
                    workspace_code
                        .as_deref()
                        .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
                    resource_code
                        .as_deref()
                        .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
                )
                .map_err(UploadParseError::Resource)?;
                resources
                    .preflight_binary_upload(actor_user_code, &command)
                    .map_err(UploadParseError::Resource)?;
                let file_name = field
                    .file_name()
                    .map(str::to_owned)
                    .filter(|name| !name.trim().is_empty())
                    .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
                let mut output =
                    tokio::fs::File::from_std(tempfile().map_err(|_| UploadParseError::Internal)?);
                let mut received = 0_u64;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|error| UploadParseError::Malformed(error.status()))?
                {
                    received = received
                        .checked_add(chunk.len() as u64)
                        .ok_or(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE))?;
                    if received > max_binary_bytes {
                        return Err(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE));
                    }
                    output
                        .write_all(&chunk)
                        .await
                        .map_err(|_| UploadParseError::Internal)?;
                }
                output
                    .flush()
                    .await
                    .map_err(|_| UploadParseError::Internal)?;
                file = Some((command, file_name, output.into_std().await));
            }
            _ => return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST)),
        }
    }

    let (command, file_name, file) =
        file.ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
    Ok(UploadBinaryForm {
        command,
        file_name,
        file,
    })
}

async fn read_upload_text(
    mut field: axum::extract::multipart::Field<'_>,
) -> Result<String, UploadParseError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|error| UploadParseError::Malformed(error.status()))?
    {
        let length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE))?;
        if length > MAX_UPLOAD_CONTROL_FIELD_BYTES {
            return Err(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| UploadParseError::Malformed(StatusCode::BAD_REQUEST))
}

async fn read_upload_text_with_limit(
    mut field: axum::extract::multipart::Field<'_>,
    maximum: usize,
) -> Result<String, UploadParseError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|error| UploadParseError::Malformed(error.status()))?
    {
        let length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE))?;
        if length > maximum {
            return Err(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| UploadParseError::Malformed(StatusCode::BAD_REQUEST))
}

fn content_disposition(
    file_name: &str,
) -> Result<HeaderValue, axum::http::header::InvalidHeaderValue> {
    let mut encoded = String::new();
    for byte in file_name.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                *byte,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            )
        {
            encoded.push(*byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}"))
}

pub fn upload_body_limit(max_binary_bytes: u64) -> DefaultBodyLimit {
    let overhead = 1_048_576_u64;
    let maximum = max_binary_bytes
        .saturating_add(overhead)
        .min(usize::MAX as u64) as usize;
    DefaultBodyLimit::max(maximum)
}

pub fn commit_body_limit(max_commit_bytes: u64) -> DefaultBodyLimit {
    let overhead = 1_048_576_u64;
    let maximum = max_commit_bytes
        .saturating_add(overhead)
        .min(usize::MAX as u64) as usize;
    DefaultBodyLimit::max(maximum)
}
