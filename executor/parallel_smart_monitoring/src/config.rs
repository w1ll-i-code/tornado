use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct ParallelSmartMonitoringConfig {
    pub max_parallel_requests: usize,
}
