use std::{collections::BTreeSet, fs::File};

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
        ResourceCommitBinary, ResourceCommitCommand, ResourceDefinitionDiscardDraftCommand,
        ResourceDefinitionPublishDraftCommand, ResourceDefinitionUpdateDraftCommand,
        ResourceDiscardDraftCommand, ResourceHttpBinaryDownloadCommand,
        ResourceHttpBinaryRemoveCommand, ResourcePublishDraftCommand, ResourceService,
        ResourceUpdateDraftCommand,
    },
    domain::resource::ResourceError,
};

use super::{
    DefinitionActivateVersionRequest, DefinitionActivationResponse, DefinitionCreateDraftRequest,
    DefinitionDeleteRequest, DefinitionDeleteResponse, DefinitionDraftActionResponse,
    DefinitionDraftTargetRequest, DefinitionGetRequest, DefinitionPageRequest,
    DefinitionSetEnabledRequest, DefinitionUpdateDraftRequest, DefinitionVersionPageResponse,
    DefinitionVersionResponse, ResourceActivateVersionRequest, ResourceActivationResponse,
    ResourceBinaryActionResponse, ResourceBinaryDownloadRequest, ResourceBinaryRemoveRequest,
    ResourceCommitRequest, ResourceCreateDraftRequest, ResourceDeleteRequest,
    ResourceDeleteResponse, ResourceDraftActionResponse, ResourceDraftTargetRequest,
    ResourceGetRequest, ResourceHttpApiResponse, ResourcePageRequest, ResourceUpdateDraftRequest,
    ResourceVersionPageResponse, ResourceVersionResponse, binary_upload_command,
    error_mapper::{internal_error_response, malformed_request_response, resource_error_response},
};

pub async fn page_definitions(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionPageRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.page_definition_versions(&auth.user().user_code, request.into()) {
        Ok(page) => Json(ResourceHttpApiResponse::success(
            DefinitionVersionPageResponse::from(page),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn get_definition(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionGetRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.get_definition_version(&auth.user().user_code, request.into()) {
        Ok(record) => Json(ResourceHttpApiResponse::success(
            DefinitionVersionResponse::from(record),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn create_definition_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionCreateDraftRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.create_definition_draft_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionDraftActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn update_definition_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionUpdateDraftRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceDefinitionUpdateDraftCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.update_definition_draft_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionDraftActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn discard_definition_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionDraftTargetRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceDefinitionDiscardDraftCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.discard_definition_draft_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionDraftActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn publish_definition_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionDraftTargetRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceDefinitionPublishDraftCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.publish_definition_draft_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn activate_definition_version(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionActivateVersionRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.activate_definition_version_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn set_definition_enabled(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionSetEnabledRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.set_definition_enabled_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn delete_definition(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<DefinitionDeleteRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.delete_definition_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            DefinitionDeleteResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

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
    match resources.page_resource_versions(&auth.user().user_code, request.into()) {
        Ok(page) => Json(ResourceHttpApiResponse::success(
            ResourceVersionPageResponse::from(page),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
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
    match resources.get_resource_version(&auth.user().user_code, request.into()) {
        Ok(record) => Json(ResourceHttpApiResponse::success(
            ResourceVersionResponse::from(record),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn create_resource(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Multipart, MultipartRejection>,
) -> Response {
    commit_resource(resources, auth, request, true).await
}

pub async fn update_resource(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Multipart, MultipartRejection>,
) -> Response {
    commit_resource(resources, auth, request, false).await
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
    let command = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.delete_resource_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceDeleteResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn create_resource_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceCreateDraftRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.create_resource_draft_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceDraftActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn update_resource_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceUpdateDraftRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceUpdateDraftCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.update_resource_draft_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceDraftActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn discard_resource_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceDraftTargetRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceDiscardDraftCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.discard_resource_draft_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceDraftActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn publish_resource_draft(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceDraftTargetRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourcePublishDraftCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.publish_resource_draft_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
    }
}

pub async fn activate_resource_version(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceActivateVersionRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match resources.activate_resource_version_for_http(&auth.user().user_code, request.into()) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceActivationResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
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
        Err(UploadParseError::Resource(error)) => return resource_error_response(error, &locale),
        Err(UploadParseError::Internal) => return internal_error_response(&locale),
    };
    let service = resources.clone();
    match tokio::task::spawn_blocking(move || {
        let mut file = form.file;
        service.upload_http_binary_from_file(&actor_user_code, form.command, &mut file)
    })
    .await
    {
        Ok(Ok(result)) => Json(ResourceHttpApiResponse::success(
            ResourceBinaryActionResponse::from(result),
        ))
        .into_response(),
        Ok(Err(error)) => resource_error_response(error, &locale),
        Err(_) => internal_error_response(&locale),
    }
}

async fn commit_resource(
    resources: ResourceService,
    auth: RequestAuthContext,
    request: Result<Multipart, MultipartRejection>,
    create: bool,
) -> Response {
    let locale = auth.locale.clone();
    let multipart = match request {
        Ok(multipart) => multipart,
        Err(error) => return malformed_request_response(error.status(), &locale),
    };
    let actor_user_code = auth.user().user_code.clone();
    let form = match parse_resource_commit_form(multipart, &resources, &actor_user_code, create)
        .await
    {
        Ok(form) => form,
        Err(UploadParseError::Malformed(status)) => {
            return malformed_request_response(status, &locale);
        }
        Err(UploadParseError::Resource(error)) => return resource_error_response(error, &locale),
        Err(UploadParseError::Internal) => return internal_error_response(&locale),
    };
    let service = resources.clone();
    match tokio::task::spawn_blocking(move || {
        if create {
            service.create_resource_for_http(&actor_user_code, form.command, form.binaries)
        } else {
            service.update_resource_for_http(&actor_user_code, form.command, form.binaries)
        }
    })
    .await
    {
        Ok(Ok(result)) => Json(ResourceHttpApiResponse::success(
            ResourceActivationResponse::from(result),
        ))
        .into_response(),
        Ok(Err(error)) => resource_error_response(error, &locale),
        Err(_) => internal_error_response(&locale),
    }
}

pub async fn remove_binary(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    request: Result<Json<ResourceBinaryRemoveRequest>, JsonRejection>,
) -> Response {
    let locale = auth.locale.clone();
    let request = match json_body(request, &locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command: ResourceHttpBinaryRemoveCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => return resource_error_response(error, &locale),
    };
    match resources.remove_http_binary(&auth.user().user_code, command) {
        Ok(result) => Json(ResourceHttpApiResponse::success(
            ResourceBinaryActionResponse::from(result),
        ))
        .into_response(),
        Err(error) => resource_error_response(error, &locale),
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
    let command: ResourceHttpBinaryDownloadCommand = request.into();
    let file = match tempfile() {
        Ok(file) => file,
        Err(_) => return internal_error_response(&locale),
    };
    let actor_user_code = auth.user().user_code.clone();
    let service = resources.clone();
    let (download, file) = match tokio::task::spawn_blocking(move || {
        let mut file = file;
        let download =
            service.download_http_binary_to_file(&actor_user_code, command, &mut file)?;
        Ok::<_, ResourceError>((download, file))
    })
    .await
    {
        Ok(Ok(download)) => download,
        Ok(Err(error)) => return resource_error_response(error, &locale),
        Err(_) => return internal_error_response(&locale),
    };
    let file = tokio::fs::File::from_std(file);
    let content_length = download.byte_size;
    let file_name = download.binary_name;
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

fn json_body<T>(request: Result<Json<T>, JsonRejection>, locale: &str) -> Result<T, Response> {
    request
        .map(|Json(value)| value)
        .map_err(|error| malformed_request_response(error.status(), locale))
}

struct UploadBinaryForm {
    command: crate::application::resource::ResourceHttpBinaryUploadCommand,
    file: File,
}

struct ResourceCommitForm {
    command: ResourceCommitCommand,
    binaries: Vec<ResourceCommitBinary>,
}

enum UploadParseError {
    Malformed(StatusCode),
    Resource(ResourceError),
    Internal,
}

const MAX_UPLOAD_CONTROL_FIELD_BYTES: usize = 8 * 1024;

async fn parse_upload_form(
    mut multipart: Multipart,
    max_binary_bytes: u64,
    resources: &ResourceService,
    actor_user_code: &str,
) -> Result<UploadBinaryForm, UploadParseError> {
    let mut seen = BTreeSet::new();
    let mut scope_kind = None;
    let mut workspace_code = None;
    let mut resource_code = None;
    let mut resource_version = None;
    let mut expected_draft_revision = None;
    let mut binary_name = None;
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
            // Every control field must arrive before the binary field.  This
            // lets us authorize the exact target before allocating a file.
            return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST));
        }
        match name.as_str() {
            "scopeKind" => scope_kind = Some(read_upload_text(field).await?),
            "workspaceCode" => workspace_code = Some(read_upload_text(field).await?),
            "resourceCode" => resource_code = Some(read_upload_text(field).await?),
            "resourceVersion" => resource_version = Some(read_upload_text(field).await?),
            "expectedDraftRevision" => {
                expected_draft_revision = Some(read_upload_text(field).await?)
            }
            "binaryName" => binary_name = Some(read_upload_text(field).await?),
            "file" => {
                let command = upload_command_from_fields(
                    &scope_kind,
                    &workspace_code,
                    &resource_code,
                    &resource_version,
                    &expected_draft_revision,
                    &binary_name,
                )?;
                resources
                    .preflight_http_binary_upload(actor_user_code, &command)
                    .map_err(UploadParseError::Resource)?;
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
                file = Some((command, output.into_std().await));
            }
            _ => return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST)),
        }
    }

    let (command, file) = file.ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
    Ok(UploadBinaryForm { command, file })
}

async fn parse_resource_commit_form(
    mut multipart: Multipart,
    resources: &ResourceService,
    actor_user_code: &str,
    create: bool,
) -> Result<ResourceCommitForm, UploadParseError> {
    let mut command = None;
    let mut binaries = Vec::new();
    let mut binary_names = BTreeSet::new();
    let mut received_total = 0_u64;
    let mut seen_binary = false;

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
            "command" if !seen_binary && command.is_none() => {
                let value =
                    read_upload_text_with_limit(field, resources.max_commit_control_bytes())
                        .await?;
                let request: ResourceCommitRequest = serde_json::from_str(&value)
                    .map_err(|_| UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
                let parsed: ResourceCommitCommand = request.into();
                resources
                    .preflight_resource_commit(actor_user_code, &parsed, create)
                    .map_err(UploadParseError::Resource)?;
                command = Some(parsed);
            }
            "binary" => {
                let _ = command
                    .as_ref()
                    .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
                seen_binary = true;
                if binaries.len() >= resources.max_commit_binaries() {
                    return Err(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE));
                }
                let binary_name = field
                    .file_name()
                    .map(str::to_owned)
                    .filter(|name| !name.trim().is_empty())
                    .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
                if !binary_names.insert(binary_name.clone()) {
                    return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST));
                }
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
                    received_total = received_total
                        .checked_add(chunk.len() as u64)
                        .ok_or(UploadParseError::Malformed(StatusCode::PAYLOAD_TOO_LARGE))?;
                    if received > resources.max_binary_bytes()
                        || received_total > resources.max_commit_bytes()
                    {
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
                binaries.push(ResourceCommitBinary {
                    binary_name,
                    source: output.into_std().await,
                });
            }
            _ => return Err(UploadParseError::Malformed(StatusCode::BAD_REQUEST)),
        }
    }
    let command = command.ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?;
    Ok(ResourceCommitForm { command, binaries })
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

fn upload_command_from_fields(
    scope_kind: &Option<String>,
    workspace_code: &Option<String>,
    resource_code: &Option<String>,
    resource_version: &Option<String>,
    expected_draft_revision: &Option<String>,
    binary_name: &Option<String>,
) -> Result<crate::application::resource::ResourceHttpBinaryUploadCommand, UploadParseError> {
    binary_upload_command(
        scope_kind
            .as_deref()
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
        workspace_code.clone(),
        resource_code
            .clone()
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
        resource_version
            .clone()
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
        expected_draft_revision
            .as_deref()
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
        binary_name
            .clone()
            .ok_or(UploadParseError::Malformed(StatusCode::BAD_REQUEST))?,
    )
    .map_err(UploadParseError::Resource)
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
