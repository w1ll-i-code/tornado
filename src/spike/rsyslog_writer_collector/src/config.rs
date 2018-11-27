use config_rs::{Config, ConfigError, Environment, File};
use std::collections::HashMap;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, Deserialize)]
pub struct Io {
    pub json_events_path: String,
    pub repeat_send: usize,
    pub repeat_sleep_ms: u64
}

#[derive(Debug, Deserialize)]
pub struct Conf {
    pub logger: LoggerConfig,
    pub io: Io,
}

impl Conf {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
/*
        s.set_default("io.uds_socket_mailbox_capacity", 10000 )?;
        s.set_default("io.uds_socket_path", "/tmp/tornado_spike_actix" )?;

        s.set_default("logger.root_level", "info" )?;
        s.set_default("logger.output_system_enabled", false )?;
        s.set_default("logger.output_file_enabled", false )?;
        s.set_default("logger.output_file_name", "" )?;
        s.set_default("logger.module_level", HashMap::<String, String>::new() )?;
*/
        s.merge(File::with_name("config/config"))?;
        s.merge(Environment::with_prefix("engine"))?;
        s.try_into()
    }
}
