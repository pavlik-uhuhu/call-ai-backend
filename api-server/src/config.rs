use std::{net::SocketAddr, time::Duration};

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config {
    pub db: DbConnectionConfig,
    pub http: HttpConfig,
    pub worker_app: HttpClientConfig,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct DbConnectionConfig {
    pub size: u32,
    pub idle_size: Option<u32>,
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub max_lifetime: Duration,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpConfig {
    pub api_listener_address: SocketAddr,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpClientConfig {
    pub url: String,
    #[serde(with = "humantime_serde")]
    pub timeout: Option<Duration>,
}

pub fn load() -> Result<Config, config::ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name("App"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()?
        .try_deserialize()
}
