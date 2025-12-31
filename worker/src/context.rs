use async_trait::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};

use crate::clients::speech_recognition::{HttpSpeechRecognitionClient, SpeechRecognitionClient};
use crate::config::Config;
use crate::indexer::{Indexer, TantivyIndexer};

#[async_trait]
pub trait Context {
    type Indexer: Indexer;
    type SpeechRecognitionClient: SpeechRecognitionClient;

    fn indexer(&self) -> &Self::Indexer;
    fn speech_recognition(&self) -> &Self::SpeechRecognitionClient;
    async fn get_db_conn(&self) -> anyhow::Result<PoolConnection<Postgres>>;
}

#[derive(Clone)]
pub struct AppContext {
    db: PgPool,
    indexer: TantivyIndexer,
    speech_recognition: HttpSpeechRecognitionClient,
}

impl AppContext {
    pub fn new(config: &Config, pool: PgPool) -> anyhow::Result<Self> {
        Ok(Self {
            db: pool,
            indexer: TantivyIndexer::new(&config.index_path)?,
            speech_recognition: HttpSpeechRecognitionClient::new(&config.speech_recognition)?,
        })
    }
}

#[async_trait]
impl Context for AppContext {
    type Indexer = TantivyIndexer;
    type SpeechRecognitionClient = HttpSpeechRecognitionClient;

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
