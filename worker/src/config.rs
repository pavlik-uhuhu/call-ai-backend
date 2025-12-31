use std::{net::SocketAddr, time::Duration};

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config {
    pub speech_recognition: HttpClientConfig,
    pub db: DbConnectionConfig,
    pub http: HttpConfig,
    pub index_path: String,
    pub amqp_prefetch_count: u16, // in-flight count
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpClientConfig {
    pub url: String,
    #[serde(with = "humantime_serde")]
    pub timeout: Option<Duration>,
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
    pub internal_api_listener_address: SocketAddr,
}

pub fn load() -> Result<Config, config::ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name("App"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()?
        .try_deserialize()
}
