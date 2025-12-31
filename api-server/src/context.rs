use async_trait::async_trait;
use lapin::options::BasicPublishOptions;
use lapin::{BasicProperties, Channel};
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};

use crate::clients::worker::{HttpWorkerClient, WorkerClient};
use crate::config::Config;
use crate::error::{Error, ErrorExt, ErrorKind};

#[async_trait]
pub trait TaskPublisher {
    async fn publish<T: serde::Serialize + Sync>(&self, payload: &T) -> Result<(), Error>;
}

#[async_trait]
impl TaskPublisher for Channel {
    async fn publish<T: serde::Serialize + Sync>(&self, payload: &T) -> Result<(), Error> {
        self.basic_publish(
            "task_exchanger",
            "task",
            BasicPublishOptions::default(),
            &serde_json::to_vec(payload).error(ErrorKind::SerializationFailed)?,
            BasicProperties::default(),
        )
        .await
        .error(ErrorKind::AMQPError)?;

        Ok(())
    }
}

#[async_trait]
pub trait Context {
    type TaskPublisher: TaskPublisher;
    type WorkerClient: WorkerClient + Sync;

    fn publisher(&self) -> &Self::TaskPublisher;

    fn worker_client(&self) -> &Self::WorkerClient;

    async fn get_db_conn(&self) -> Result<PoolConnection<Postgres>, Error>;
}

#[derive(Clone)]
pub struct AppContext {
    db: PgPool,
    channel: Channel,
    worker_client: HttpWorkerClient,
}

impl AppContext {
    pub fn new(channel: Channel, pool: PgPool, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            db: pool,
            channel,
            worker_client: HttpWorkerClient::new(&config.worker_app)?,
        })
    }
}

#[async_trait]
impl Context for AppContext {
    type TaskPublisher = Channel;
    type WorkerClient = HttpWorkerClient;

    fn publisher(&self) -> &Self::TaskPublisher {
        &self.channel
    }

    fn worker_client(&self) -> &Self::WorkerClient {
        &self.worker_client
    }

    async fn get_db_conn(&self) -> Result<PoolConnection<Postgres>, Error> {
        let conn = self.db.acquire().await?;
        Ok(conn)
    }
}
