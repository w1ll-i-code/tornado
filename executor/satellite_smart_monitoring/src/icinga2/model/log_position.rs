use serde::{Deserialize, Serialize};

use crate::icinga2::model::IcingaTimestamp;

/// The LogPosition is the parameter of the `log::SetLogPosition` message. It contains only the
/// timestamp of the last processed message of the remote host. It can then be used to verify which
/// messages where already successfully processed by the host and can be dropped. This is useful to
/// ensure the integrity of all messages on a lost connection.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct LogPosition {
    log_position: IcingaTimestamp,
}

impl LogPosition {
    pub fn now() -> Self {
        LogPosition { log_position: Default::default() }
    }

    pub fn ts(&self) -> IcingaTimestamp {
        self.log_position
    }
}

impl From<IcingaTimestamp> for LogPosition {
    fn from(ts: IcingaTimestamp) -> Self {
        LogPosition { log_position: ts }
    }
}

impl From<f64> for LogPosition {
    fn from(ts: f64) -> Self {
        LogPosition { log_position: ts.into() }
    }
}

impl From<LogPosition> for IcingaTimestamp {
    fn from(log_position: LogPosition) -> Self {
        log_position.log_position
    }
}
