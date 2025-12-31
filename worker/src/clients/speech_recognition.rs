use async_trait::async_trait;
#[cfg(test)]
use mockall::{automock, predicate::*};
use protocol::{
    db::metadata::CallMetadata,
    entity::{speech_recog::RecognitionData, ParticipantKind},
};
use serde::Serialize;
use thiserror::Error;
use tracing::error;
use url::Url;

use crate::config::HttpClientConfig;

#[derive(Error, Debug)]
pub enum SpeechRecognitionClientError {
    #[error("failed to deserialize response of the HTTP client: {0}")]
    De(#[source] reqwest::Error),
    #[error("failed to communicate in HTTP client: {0}")]
    Channel(#[source] reqwest::Error),
    #[error("server failed to perform request of HTTP client: {0}")]
    ResponseStatus(http::StatusCode),
    #[error("failed to parse URL: {0}")]
    BaseUrl(#[source] url::ParseError),
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait SpeechRecognitionClient {
    async fn transcribe(
        &self,
        request: TranscribeRequest,
    ) -> Result<RecognitionData, SpeechRecognitionClientError>;
}

#[derive(Clone)]
pub struct HttpSpeechRecognitionClient {
    client: reqwest::Client,
    base_url: Url,
}

impl HttpSpeechRecognitionClient {
    pub fn new(config: &HttpClientConfig) -> Result<Self, SpeechRecognitionClientError> {
        let mut builder = reqwest::Client::builder();

        if let Some(timeout) = config.timeout {
            builder = builder.timeout(timeout);
        }

        builder
            .build()
            .map_err(SpeechRecognitionClientError::Channel)?;

        let base_url = Url::parse(&config.url).map_err(SpeechRecognitionClientError::BaseUrl)?;

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
        })
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TranscribeRequest {
    file_url: String,
    operator_channel: String,
    tasks: Vec<String>,
}

impl From<&CallMetadata> for TranscribeRequest {
    fn from(metadata: &CallMetadata) -> Self {
        let operator_channel = if metadata.left_channel == ParticipantKind::Employee {
            "L".to_string()
        } else {
            "R".to_string()
        };

        TranscribeRequest {
            file_url: metadata.file_url.clone(),
            operator_channel,
            tasks: vec![
                "speech_recognition".to_string(),
                "emotion_recognition".to_string(),
            ],
        }
    }
}

#[async_trait]
impl SpeechRecognitionClient for HttpSpeechRecognitionClient {
    async fn transcribe(
        &self,
        request: TranscribeRequest,
    ) -> Result<RecognitionData, SpeechRecognitionClientError> {
        let mut req_url = self.base_url.clone();
        req_url.set_path("extract_info_s3/");

        let res = self
            .client
            .post(req_url)
            .json(&request)
            .send()
            .await
            .map_err(SpeechRecognitionClientError::Channel)?;

        match res.status() {
            reqwest::StatusCode::OK => res
                .json::<RecognitionData>()
                .await
                .map_err(SpeechRecognitionClientError::De),
            otherwise => Err(SpeechRecognitionClientError::ResponseStatus(otherwise)),
        }
    }
}
