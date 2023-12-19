use std::path::PathBuf;
use std::time::Duration;
use tokio_rustls::rustls::ServerName;

pub struct IcingaSatelliteConfig {
    pub endpoint: EndPointConfig,
    pub cert_root_path: PathBuf,
    pub retry_time: Duration,
    pub max_parallel_send: usize,
    pub intern_buffer_size: usize,
}

impl IcingaSatelliteConfig {
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

pub struct EndPointConfig {
    pub address: ServerName,
    pub port: u16,
    pub host_name: String,
    pub ticket: String,
}
