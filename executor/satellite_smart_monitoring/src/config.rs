use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt::Formatter;
use std::path::PathBuf;
use std::time::Duration;
use tokio_rustls::rustls::ServerName;

#[derive(Deserialize, Clone)]
pub struct SatelliteSmartMonitoringConfig {
    pub endpoint: EndPointConfig,
    pub cert_root_path: PathBuf,
    pub retry_time: Duration,
    pub max_parallel_send: usize,
    pub intern_buffer_size: usize,
}

impl SatelliteSmartMonitoringConfig {
    pub fn ca_path(&self) -> PathBuf {
        let mut path = self.cert_root_path.clone();
        path.push("ca.crt");
        path
    }

    pub fn cert_path(&self) -> PathBuf {
        let mut path = self.cert_root_path.clone();
        path.push(format!("{}.crt", &self.endpoint.host_name));
        path
    }

    pub fn key_path(&self) -> PathBuf {
        let mut path = self.cert_root_path.clone();
        path.push(format!("{}.key", &self.endpoint.host_name));
        path
    }

    pub fn fpr_path(&self) -> PathBuf {
        let mut path = self.cert_root_path.clone();
        path.push(format!("{}.fingerprint", &self.endpoint.host_name));
        path
    }
}

#[derive(Deserialize, Clone)]
pub struct EndPointConfig {
    #[serde(deserialize_with = "deserialize_server_name")]
    pub address: ServerName,
    pub port: u16,
    pub host_name: String,
    pub ticket: String,
}

fn deserialize_server_name<'de, D>(deserializer: D) -> Result<ServerName, D::Error>
where
    D: Deserializer<'de>,
{
    struct ServerNameVisitor;

    impl Visitor<'_> for ServerNameVisitor {
        type Value = ServerName;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("Expected valid endpoint for the servername")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            match ServerName::try_from(v) {
                Ok(server_name) => Ok(server_name),
                Err(err) => Err(E::custom(err.to_string())),
            }
        }
    }

    deserializer.deserialize_str(ServerNameVisitor)
}
