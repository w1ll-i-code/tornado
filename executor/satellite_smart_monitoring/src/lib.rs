use crate::config::IcingaSatelliteConfig;
use crate::satellite::{worker, SetStateRequest};
use flume::{SendError, Sender};
use serde_json::Error;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, OnceLock};
use tokio::task::JoinHandle;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatelessExecutor};

pub mod config;
mod error;
mod icinga2;
pub mod satellite;

struct SatelliteSmartMonitoringExecutor {
    handle: JoinHandle<Infallible>,
    config: IcingaSatelliteConfig,
    sender: Sender<SetStateRequest>,
}

impl SatelliteSmartMonitoringExecutor {
    pub async fn new(config: IcingaSatelliteConfig) -> Self {
        let (worker_tx, worker_rx) = flume::bounded(0);
        let handle = tokio::spawn(worker(config.clone(), worker_rx));
        Self { handle, config, sender: worker_tx }
    }

    pub fn get_set_state_request(action: &Action) -> Result<SetStateRequest, ExecutorError> {
        let result = serde_json::from_value(serde_json::Value::Object(action.payload.clone()));
        match result {
            Ok(request) => Ok(request),
            Err(err) => Err(ExecutorError::ConfigurationError { message: err.to_string() }),
        }
    }
}

impl Display for SatelliteSmartMonitoringExecutor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("SatelliteSmartMonitoringExecutor")
    }
}

#[async_trait::async_trait]
impl StatelessExecutor for SatelliteSmartMonitoringExecutor {
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        let message = SatelliteSmartMonitoringExecutor::get_set_state_request(&action)?;
        match self.sender.send_async(message).await {
            Ok(_) => Ok(()),
            Err(err) => Err(ExecutorError::ActionExecutionError {
                message: "Worker terminated unexpectedly. Can't perform any further actions."
                    .to_string(),
                can_retry: false,
                code: None,
                data: Default::default(),
            }),
        }
    }
}
