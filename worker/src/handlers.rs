use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use http::StatusCode;
use uuid::Uuid;

use crate::{
    context::{AppContext, Context},
    indexer::Indexer,
};

pub fn int_api_router(cx: AppContext) -> Router {
    Router::new().nest(
        "/api/v1",
        Router::new()
            .route("/transcript/:id", get(transcript))
            .with_state(cx),
    )
}

pub async fn transcript(State(cx): State<AppContext>, Path(id): Path<Uuid>) -> Response {
    let payload = match cx.indexer().load_transcript_payload(id).await {
        Ok(bytes) => bytes,
        Err(err) => return err.into_response(),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload))
        .expect("http body bytes payload")
}
