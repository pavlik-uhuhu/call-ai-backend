use std::future::IntoFuture;

use anyhow::{Context as _, Result};
use futures::{future, future::TryFutureExt, StreamExt};
use lapin::{Connection, ConnectionProperties};
use signal_hook::consts::TERM_SIGNALS;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::{info, warn};

use crate::config::DbConnectionConfig;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = config::load().context("Failed to load config")?;
    info!("App config: {:?}", config);

    let amqp_connection = create_broker_connection().await?;
    let amqp_channel = amqp_connection.create_channel().await?;
    let pool = create_pool(&config.db).await?;
    let cx = crate::context::AppContext::new(amqp_channel, pool, config.clone())?;

    let api_listener = tokio::net::TcpListener::bind(&config.http.api_listener_address).await?;
    let api_handle = tokio::spawn(
        axum::serve(api_listener, crate::handlers::api_router(cx))
            .into_future()
            .map_err(anyhow::Error::from),
    );

    let mut signals_stream = signal_hook_tokio::Signals::new(TERM_SIGNALS)?.fuse();
    let signals_handle = tokio::spawn(async move {
        let _ = signals_stream.next().await;
        let res: Result<()> = Ok(());
        res
    });

    let (result, number, _) = future::select_all(vec![api_handle, signals_handle]).await;
    let context = format!("Error from call ai handle #{number}");
    let result = result.context("Join error on handlers")?.context(context);
    if let Err(err) = &result {
        warn!("{err}");
    } else {
        warn!("Call ai app has finished");
    }

    result
}

async fn create_broker_connection() -> anyhow::Result<lapin::Connection> {
    let url = std::env::var("RABBITMQ_URL")?;
    let options = ConnectionProperties::default();
    let connection = Connection::connect(&url, options).await?;

    Ok(connection)
}

pub async fn create_pool(config: &DbConnectionConfig) -> Result<PgPool> {
    let url = std::env::var("DATABASE_URL")?;
    let res = PgPoolOptions::new()
        .max_connections(config.size)
        .min_connections(config.idle_size.unwrap_or(1))
        .acquire_timeout(config.timeout)
        .max_lifetime(config.max_lifetime)
        .connect(&url)
        .await?;

    Ok(res)
}

mod clients;
mod config;
mod context;
mod db;
mod error;
mod handlers;
#[cfg(test)]
mod test_helpers;
