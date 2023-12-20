use crate::config::SatelliteSmartMonitoringConfig;
use crate::satellite::{worker, SetStateRequest};
use flume::Sender;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatelessExecutor};

pub mod config;
mod error;
mod icinga2;
pub mod satellite;

#[allow(dead_code)] // Allow handle to be unused in this struct.
pub struct SatelliteSmartMonitoringExecutor {
    handle: JoinHandle<Infallible>,
    sender: Sender<SetStateRequest>,
}

impl SatelliteSmartMonitoringExecutor {
    pub fn new(config: SatelliteSmartMonitoringConfig) -> Self {
        let (worker_tx, worker_rx) = flume::bounded(0);
        let handle = tokio::spawn(worker(config.clone(), worker_rx));
        Self { handle, sender: worker_tx }
    }

    fn get_set_state_request(action: &Action) -> Result<SetStateRequest, ExecutorError> {
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
            Err(_) => Err(ExecutorError::ActionExecutionError {
                message: "Worker terminated unexpectedly. Can't perform any further actions."
                    .to_string(),
                can_retry: false,
                code: None,
                data: Default::default(),
            }),
        }
    }
}
