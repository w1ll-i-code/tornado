use clap::{App, Arg, ArgMatches};
use config_rs::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common_logger::LoggerConfig;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> =
    option_env!("TORNADO_EMAIL_COLLECTOR_CONFIG_DIR_DEFAULT");

pub fn arg_matches() -> ArgMatches {
    App::new("tornado_email_collector")
        .arg(Arg::new("config-dir")
            .long("config-dir")
            .about("The filesystem folder where the Tornado Email Collector configuration is saved")
            .default_value(CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado_email_collector")))
        .get_matches()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub email_collector: EmailCollectorConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct EmailCollectorConfig {
    pub message_queue_size: usize,
    pub uds_path: String,

    pub tornado_connection_channel: Option<TornadoConnectionChannel>,

    pub tornado_event_socket_ip: Option<String>,
    pub tornado_event_socket_port: Option<u16>,
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let config_file_path = format!("{}/{}", config_dir, "email_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_read_configuration_from_file() {
        // Arrange
        let path = "./config/";

        // Act
        let config = build_config(path);

        // Assert
        assert!(config.is_ok())
    }
}
