use async_trait::async_trait;
use bytes::Bytes;
#[cfg(test)]
use mockall::{automock, predicate::*};
use protocol::entity::speech_recog::RecognitionData;
use thiserror::Error;
use tracing::error;
use url::Url;
use uuid::Uuid;

use crate::config::HttpClientConfig;

#[derive(Error, Debug)]
pub enum WorkerClientError {
    #[error("failed to deserialize response of the HTTP client: {0}")]
    De(#[source] serde_json::Error),
    #[error("failed to communicate in HTTP client: {0}")]
    Channel(#[source] reqwest::Error),
    #[error("server failed to perform request of HTTP client: {0}")]
    ResponseStatus(http::StatusCode),
    #[error("failed to parse URL: {0}")]
    BaseUrl(#[source] url::ParseError),
    #[error("http reqwest error: {0}")]
    ReqwestError(#[source] reqwest::Error),
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait WorkerClient {
    async fn raw_transcript_by_id(&self, task_id: Uuid) -> Result<Bytes, WorkerClientError>;
    async fn transcript_by_id(&self, task_id: Uuid) -> Result<RecognitionData, WorkerClientError> {
        let bytes_res = self.raw_transcript_by_id(task_id).await?;
        let recog_data = serde_json::from_slice(&bytes_res).map_err(WorkerClientError::De)?;
        Ok(recog_data)
    }
}

#[derive(Clone)]
pub struct HttpWorkerClient {
    client: reqwest::Client,
    base_url: Url,
}

impl HttpWorkerClient {
    pub fn new(config: &HttpClientConfig) -> Result<Self, WorkerClientError> {
        let mut builder = reqwest::Client::builder();

        if let Some(timeout) = config.timeout {
            builder = builder.timeout(timeout);
        }

        builder.build().map_err(WorkerClientError::Channel)?;

        let base_url = Url::parse(&config.url).map_err(WorkerClientError::BaseUrl)?;

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
        })
    }
}

#[async_trait]
impl WorkerClient for HttpWorkerClient {
    async fn raw_transcript_by_id(&self, task_id: Uuid) -> Result<Bytes, WorkerClientError> {
        let mut req_url = self.base_url.clone();
        req_url.set_path(&format!("api/v1/transcript/{task_id}"));

        let res = self
            .client
            .get(req_url)
            .send()
            .await
            .map_err(WorkerClientError::Channel)?;

        match res.status() {
            reqwest::StatusCode::OK => res.bytes().await.map_err(WorkerClientError::ReqwestError),
            otherwise => Err(WorkerClientError::ResponseStatus(otherwise)),
        }
    }
}
