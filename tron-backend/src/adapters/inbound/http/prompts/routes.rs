use axum::{Router, routing::post};

use crate::app::AppState;

use super::handler::{analyze_template, translate_draft};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/prompts/analyze-template", post(analyze_template))
        .route("/api/v1/prompts/translate-draft", post(translate_draft))
}
