use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{pool::PoolConnection, PgPool, Postgres};

use crate::{clients::speech_recognition::MockSpeechRecognitionClient, indexer::TantivyIndexer};

#[derive(Clone)]
pub struct TestContext {
    db: PgPool,
    indexer: TantivyIndexer,
    speech_recognition: Arc<MockSpeechRecognitionClient>,
}

impl TestContext {
    pub async fn new(db: PgPool) -> Self {
        Self {
            db,
            indexer: TantivyIndexer::new("").expect("failed to create indexer"),
            speech_recognition: Arc::new(MockSpeechRecognitionClient::new()),
        }
    }

    pub fn speech_recog_client_mock(&mut self) -> &mut MockSpeechRecognitionClient {
        Arc::get_mut(&mut self.speech_recognition).unwrap()
    }
}

#[async_trait]
impl crate::context::Context for TestContext {
    type Indexer = TantivyIndexer;
    type SpeechRecognitionClient = MockSpeechRecognitionClient;

    fn indexer(&self) -> &Self::Indexer {
        &self.indexer
    }

    fn speech_recognition(&self) -> &Self::SpeechRecognitionClient {
        &self.speech_recognition
    }

    async fn get_db_conn(&self) -> anyhow::Result<PoolConnection<Postgres>> {
        let conn = self.db.acquire().await?;
        Ok(conn)
    }
}
