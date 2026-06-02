use std::path::{Path, PathBuf};

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};

use crate::{
    app::AppState,
    config::AppConfig,
    response::{ApiResponse, EmptyData},
};

pub fn routes() -> Router<AppState> {
    Router::new().fallback(frontend_resource)
}

fn api_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::<EmptyData>::error(
            "NOT_FOUND",
            "API route not found",
        )),
    )
        .into_response()
}

async fn frontend_resource(State(config): State<AppConfig>, uri: Uri) -> Response {
    if is_api_path(uri.path()) {
        return api_not_found();
    }

    let root = PathBuf::from(&config.frontend_dist_dir);
    let uri_path = uri.path();

    if let Some(candidate) = frontend_file_path(&root, uri_path) {
        if is_file(&candidate).await {
            return file_response(candidate).await;
        }
        if looks_like_file_request(uri_path) {
            return frontend_not_found().into_response();
        }
    }

    file_response(root.join("index.html")).await
}

fn is_api_path(uri_path: &str) -> bool {
    uri_path == "/api" || uri_path.starts_with("/api/")
}

fn frontend_file_path(root: &Path, uri_path: &str) -> Option<PathBuf> {
    let relative = uri_path.trim_start_matches('/');
    if relative.is_empty() {
        return None;
    }

    let mut path = root.to_path_buf();
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains('\\') {
            return None;
        }
        path.push(segment);
    }

    Some(path)
}

async fn is_file(path: &Path) -> bool {
    match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    }
}

async fn file_response(path: PathBuf) -> Response {
    match tokio::fs::read(&path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type(&path))], bytes).into_response(),
        Err(_) => frontend_not_found().into_response(),
    }
}

fn frontend_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "frontend resource not found",
    )
}

fn looks_like_file_request(uri_path: &str) -> bool {
    uri_path
        .rsplit('/')
        .next()
        .and_then(|segment| Path::new(segment).extension())
        .is_some()
}

fn content_type(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return "application/octet-stream";
    };

    match extension.to_ascii_lowercase().as_str() {
        "css" => "text/css; charset=utf-8",
        "gif" => "image/gif",
        "html" | "htm" => "text/html; charset=utf-8",
        "ico" => "image/x-icon",
        "jpg" | "jpeg" => "image/jpeg",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "map" => "application/json; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        "wasm" => "application/wasm",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}
