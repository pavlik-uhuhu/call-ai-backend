use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tokio::sync::Mutex;

use crate::{
    clients::worker::MockWorkerClient,
    config::Config,
    context::TaskPublisher,
    error::{Error, ErrorExt, ErrorKind},
};

#[derive(Clone)]
pub struct TestContext {
    db: PgPool,
    _config: Config,
    publisher: Arc<TestPublisher>,
    worker_client: Arc<MockWorkerClient>,
}

impl TestContext {
    pub async fn new(db: PgPool) -> Self {
        Self {
            db,
            _config: build_config(),
            publisher: Arc::new(TestPublisher::new()),
            worker_client: Arc::new(MockWorkerClient::new()),
        }
    }

    pub fn _config(&self) -> &Config {
        &self._config
    }

    pub fn test_publisher(&self) -> &TestPublisher {
        self.publisher.as_ref()
    }

    pub fn worker_client_mock(&mut self) -> &mut MockWorkerClient {
        Arc::get_mut(&mut self.worker_client).unwrap()
    }
}

fn build_config() -> Config {
    let config = serde_json::json!({
        "http": {
            "api_listener_address": "0.0.0.0:8088"
        },
        "worker_app": {
            "url": "0.0.0.0:8087",
            "timeout": "5m"
        },
        "db": {
            "size": 5,
            "timeout": "5s",
            "max_lifetime": "12h"
        },
    });

    serde_json::from_value::<Config>(config).expect("Failed to parse test config")
}

#[async_trait]
impl crate::context::Context for TestContext {
    type WorkerClient = MockWorkerClient;
    type TaskPublisher = TestPublisher;

    fn publisher(&self) -> &Self::TaskPublisher {
        self.publisher.as_ref()
    }

    fn worker_client(&self) -> &Self::WorkerClient {
        self.worker_client.as_ref()
    }

    async fn get_db_conn(&self) -> Result<PoolConnection<Postgres>, Error> {
        let conn = self.db.acquire().await?;
        Ok(conn)
    }
}

pub struct TestPublisher {
    messages: Mutex<Vec<Value>>,
}

impl TestPublisher {
    fn new() -> Self {
        Self {
            messages: Mutex::new(vec![]),
        }
    }

    pub async fn flush(&self) -> Vec<Value> {
        let mut messages_lock = self.messages.lock().await;

        (*messages_lock).drain(0..).collect::<Vec<_>>()
    }
}

#[async_trait]
impl TaskPublisher for TestPublisher {
    async fn publish<T: serde::Serialize + Sync>(&self, payload: &T) -> Result<(), Error> {
        let serialized = serde_json::to_value(payload).error(ErrorKind::SerializationFailed)?;

        let mut messages_lock = self.messages.lock().await;

        (*messages_lock).push(serialized);
        Ok(())
    }
}
