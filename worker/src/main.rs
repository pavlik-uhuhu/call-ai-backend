use std::future::IntoFuture;

use anyhow::{Context as _, Result};
use futures::{future, future::TryFutureExt, StreamExt};
use signal_hook::consts::TERM_SIGNALS;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::{info, warn};

use crate::config::DbConnectionConfig;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = config::load().context("Failed to load config")?;
    info!("App config: {:?}", config);

    let pool = create_pool(&config.db).await?;

    let cx = crate::context::AppContext::new(&config, pool)?;

    let broker_pipe_handle = tokio::spawn(crate::pipe::run_broker_pipe(
        cx.clone(),
        config.amqp_prefetch_count,
    ));

    let int_api_listener =
        tokio::net::TcpListener::bind(&config.http.internal_api_listener_address).await?;
    let int_api_handle = tokio::spawn(
        axum::serve(
            int_api_listener,
            crate::handlers::int_api_router(cx.clone()),
        )
        .into_future()
        .map_err(anyhow::Error::from),
    );

    let mut signals_stream = signal_hook_tokio::Signals::new(TERM_SIGNALS)?.fuse();
    let signals_handle = tokio::spawn(async move {
        let _ = signals_stream.next().await;
        let res: Result<()> = Ok(());
        res
    });

    let (result, number, _) =
        future::select_all(vec![broker_pipe_handle, int_api_handle, signals_handle]).await;
    let context = format!("Error from call ai handle #{number}");
    let result = result.context("Join error on handlers")?.context(context);
    if let Err(err) = &result {
        warn!("{err}");
    } else {
        warn!("Call ai app has finished");
    }

    result
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
mod domain;
mod handlers;
mod indexer;
mod pipe;
#[cfg(test)]
mod test_helpers;
