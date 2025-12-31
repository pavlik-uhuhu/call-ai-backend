use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use http::StatusCode;
use protocol::entity::speech_recog::RecognitionData;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::clients::worker::WorkerClient;
use crate::context::{AppContext, Context};
use crate::error::{Error, ErrorKind};

#[derive(OpenApi)]
#[openapi(
    paths(transcript, download_transcript),
    components(schemas()),
    tags(
        (name = "Transcripts", description = "API for handling transcript operations")
    )
)]
pub(super) struct ApiTranscripts;

#[utoipa::path(
    get,
    path = "/{id}",
    responses(
        (status = OK, description = "Retrieve the raw JSON transcript", body = RecognitionData),
        (status = INTERNAL_SERVER_ERROR, description = "Server error while retrieving transcript")
    ),
    params(
        ("id" = Uuid, Path, description = "Unique identifier for the transcript")
    ),
    tags = ["Transcripts"]
)]
pub async fn transcript(
    State(cx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, Error> {
    do_transcript(cx, id).await
}

async fn do_transcript<C: Context>(cx: C, id: Uuid) -> Result<Response, Error> {
    let raw_body = cx
        .worker_client()
        .raw_transcript_by_id(id)
        .await
        .map_err(|err| Error::new(ErrorKind::WorkerRequestFailed, anyhow::anyhow!(err)))?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(raw_body))
        .map_err(|err| Error::new(ErrorKind::SerializationFailed, anyhow::anyhow!(err)))?;

    Ok(response)
}

#[utoipa::path(
    get,
    path = "/{id}/download",
    responses(
        (status = OK, description = "Download the transcript as a text file", content_type = "text/plain"),
        (status = INTERNAL_SERVER_ERROR, description = "Server error while downloading transcript")
    ),
    params(
        ("id" = Uuid, Path, description = "Unique identifier for the transcript")
    ),
    tags = ["Transcripts"]
)]
pub async fn download_transcript(
    State(cx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response, Error> {
    do_download_transcript(cx, id).await
}

async fn do_download_transcript<C: Context>(cx: C, id: Uuid) -> Result<Response, Error> {
    let recog_data = cx
        .worker_client()
        .transcript_by_id(id)
        .await
        .map_err(|err| Error::new(ErrorKind::WorkerRequestFailed, anyhow::anyhow!(err)))?;
    let response =
        recog_data
            .speech_recognition_result
            .iter()
            .fold(String::new(), |acc_text, recog_item| {
                let speaker = recog_item.speaker;
                let start_interval = format_seconds(recog_item.timestamps.start as i64);
                let end_interval = format_seconds(recog_item.timestamps.end as i64);
                let text = &recog_item.text;

                acc_text + &format!("[{speaker} | {start_interval} - {end_interval}]: {text}\n")
            });

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(
            http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"transcript.txt\"",
        )
        .body(Body::from(response))
        .map_err(|err| Error::new(ErrorKind::SerializationFailed, anyhow::anyhow!(err)))?;

    Ok(response)
}

fn format_seconds(duration: i64) -> String {
    let seconds = duration % 60;
    let minutes = (duration / 60) % 60;
    let hours = (duration / 60) / 60;
    format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use protocol::entity::speech_recog::{
        CallHolds, Interval, PhraseTimestamps, SpeechRecognition,
    };
    use protocol::entity::ParticipantKind;

    use crate::test_helpers::context::TestContext;

    use super::*;

    #[sqlx::test]
    async fn transcript(pool: sqlx::PgPool) {
        let mut cx = TestContext::new(pool).await;

        cx.worker_client_mock()
            .expect_transcript_by_id()
            .with(mockall::predicate::eq(Uuid::default()))
            .returning(move |_| {
                Ok(RecognitionData {
                    call_holds: CallHolds::default(),
                    emotion_recognition_result: vec![],
                    phrase_timestamps: PhraseTimestamps::default(),
                    speech_recognition_result: vec![SpeechRecognition {
                        text: "test_text".to_string(),
                        timestamps: Interval {
                            start: 0f32,
                            end: 10f32,
                        },
                        speaker: ParticipantKind::Client,
                    }],
                })
            });

        let transcript_text_resp = do_download_transcript(cx, Uuid::default())
            .await
            .expect("failed to retrieve transcript");
        assert_eq!(transcript_text_resp.status(), StatusCode::OK);

        let transcript = axum::body::to_bytes(transcript_text_resp.into_body(), usize::MAX)
            .await
            .unwrap();

        let recog_data_res: String = String::from_utf8(transcript.to_vec()).unwrap();
        assert_eq!(
            "[Client | 00:00:00 - 00:00:10]: test_text\n",
            recog_data_res
        );
    }
}
