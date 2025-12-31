use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::context::AppContext;

#[derive(OpenApi)]
#[openapi(
    info(description = "Call-AI API", contact()),
    servers(
        (url = "http://api-server.dev.call-ai.*.com", description = "Dev server")
    ),
    nest(
        (path = "/api/v1/tasks", api = task::ApiTasks),
        (path = "/api/v1/settings", api = settings::ApiSettings),
        (path = "/api/v1/dictionaries", api = dictionary::ApiDictionaries),
        (path = "/api/v1/transcripts", api = transcript::ApiTranscripts)
    )
)]
struct ApiDoc;

pub fn api_router(cx: AppContext) -> Router {
    Router::new()
        .nest(
            "/api/v1",
            tasks_router()
                .merge(settings_router())
                .merge(transcripts_router())
                .merge(dictionaries_router()),
        )
        .with_state(cx)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::new().allow_origin(Any))
}

pub fn tasks_router() -> Router<AppContext> {
    Router::new()
        .route("/tasks", post(task::create).get(task::list))
        .route("/tasks/:id", put(task::reprocess))
        .route("/tasks/:id/detailed_metrics", get(task::detailed_metrics))
        .route("/tasks/metrics", get(task::metrics_list))
}

pub fn settings_router() -> Router<AppContext> {
    Router::new()
        .route("/settings", get(settings::settings_list))
        .route("/settings/item", post(settings::settings_item_create))
        .route(
            "/settings/item/:id",
            put(settings::settings_item_update).delete(settings::settings_item_delete),
        )
}

pub fn transcripts_router() -> Router<AppContext> {
    Router::new()
        .route("/transcripts/:id", get(transcript::transcript))
        .route(
            "/transcripts/:id/download",
            get(transcript::download_transcript),
        )
}

pub fn dictionaries_router() -> Router<AppContext> {
    Router::new()
        .route(
            "/dictionaries/:id",
            get(dictionary::dict_by_id)
                .put(dictionary::update)
                .delete(dictionary::delete),
        )
        .route(
            "/dictionaries",
            get(dictionary::list_dicts).post(dictionary::create),
        )
}

mod dictionary;
mod settings;
mod task;
mod transcript;
mod utils;
