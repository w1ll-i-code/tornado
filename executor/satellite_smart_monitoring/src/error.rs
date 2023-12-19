#![allow(clippy::enum_variant_names)]

use std::io;
use thiserror::Error;
use tokio_rustls::rustls;
use tokio_rustls::rustls::client::InvalidDnsNameError;

#[derive(Error, Debug)]
pub enum IcingaSatelliteError {
    #[error("IcingaSatelliteError::IoError ({0})")]
    IoError(#[from] io::Error),

    #[error("IcingaSatelliteError::JsonRpcError")]
    JsonRpcError(#[from] serde_json::Error),

    #[error("IcingaSatelliteError::ProtocolError ({0})")]
    ProtocolError(String),

    #[error("IcingaSatelliteError::{0}")]
    ConfigError(#[from] SatelliteConfigurationError),
}

#[derive(Error, Debug)]
pub enum SatelliteConfigurationError {
    #[error("SatelliteConfigurationError::FileIoError (Could not access {file}. {error})")]
    FileIoError { file: String, error: io::Error },

    #[error("SatelliteConfigurationError::IoError ({0})")]
    IoError(#[from] io::Error),

    #[error("SatelliteConfigurationError::PkiError")]
    PkiError,

    #[error("SatelliteConfigurationError::TlsError ({0})")]
    TlsError(#[from] rustls::Error),

    #[error("SatelliteConfigurationError::InvalidDnsNameError")]
    InvalidDnsNameError,

    #[error("SatelliteConfigurationError::ConfigurationError ({0})")]
    ConfigurationError(String),
}

impl From<InvalidDnsNameError> for SatelliteConfigurationError {
    fn from(_: InvalidDnsNameError) -> Self {
        SatelliteConfigurationError::InvalidDnsNameError
    }
}

impl From<webpki::Error> for SatelliteConfigurationError {
    fn from(_: webpki::Error) -> Self {
        SatelliteConfigurationError::PkiError
    }
}
