use crate::satellite::SetStateRequest;
use flume::Sender;
use tokio::task::{JoinError, JoinHandle};

pub mod model;
pub mod object_store;

struct SatelliteSmartMonitoring {
    worker_handle: JoinHandle<Result<(), JoinError>>,
    worker_sender: Sender<SetStateRequest>,
}
