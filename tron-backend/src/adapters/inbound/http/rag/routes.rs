use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{
    corpus_catalog, create_corpus_management, create_profile_management, debug_profile,
    debug_source, delete_corpus_management, delete_profile_management, get_corpus_management,
    get_profile_management, ingest, list_corpora_management, list_profiles_management,
    pipeline_catalog, profile_catalog, retry_index, search, update_corpus_management,
    update_profile_management, update_profile_management_post, validate_filters_management,
    verify_corpus_management, verify_profile_management,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/rag/corpora",
            get(list_corpora_management).post(create_corpus_management),
        )
        .route("/api/v1/rag/corpora/catalog", get(corpus_catalog))
        .route("/api/v1/rag/corpora/verify", post(verify_corpus_management))
        .route(
            "/api/v1/rag/corpora/{corpus_id}",
            get(get_corpus_management)
                .put(update_corpus_management)
                .delete(delete_corpus_management),
        )
        .route(
            "/api/v1/rag/profiles",
            get(list_profiles_management).post(create_profile_management),
        )
        .route("/api/v1/rag/profiles/catalog", get(profile_catalog))
        .route("/api/v1/rag/pipeline/catalog", get(pipeline_catalog))
        .route(
            "/api/v1/rag/profiles/verify",
            post(verify_profile_management),
        )
        .route(
            "/api/v1/rag/profiles/update",
            post(update_profile_management_post),
        )
        .route(
            "/api/v1/rag/profiles/{profile_id}",
            get(get_profile_management)
                .put(update_profile_management)
                .delete(delete_profile_management),
        )
        .route(
            "/api/v1/rag/filters/validate",
            post(validate_filters_management),
        )
        .route("/api/v1/rag/ingest", post(ingest))
        .route("/api/v1/rag/search", post(search))
        .route("/api/v1/rag/debug/profile", get(debug_profile))
        .route("/api/v1/rag/debug/source", get(debug_source))
        .route("/api/v1/rag/retry/index", post(retry_index))
}
