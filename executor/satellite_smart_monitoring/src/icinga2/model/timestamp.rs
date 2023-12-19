use std::fmt::{Display, Formatter};
use std::time::UNIX_EPOCH;

use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

const MICRO_SECONDS_PER_SECOND: f64 = 1_000_000f64;

/// The TimeStamp type is used handle icinga2 TimeStamps internally with a bit more grace. The
/// TimeStamps received by icinga2 are serialized as a floating point number of seconds since
/// UNIX_EPOCH, however, this creates issues if we need to compare timestamps to one another, since
/// Rust does not natively implement `Eq` for `f32` or `f64`. Since we don't want to rely on the
/// quirks of float comparisons for this task, the TimeStamp type provides us with a safe
/// alternative. It will be deserialized from the float that icinga2 sends as a timestamp and will
/// be deserialized to a float, when we communicate with the icinga2 api.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct IcingaTimestamp {
    sec: u64,
    us: u32,
}

impl IcingaTimestamp {
    pub fn now() -> IcingaTimestamp {
        Default::default()
    }

    pub fn null() -> IcingaTimestamp {
        IcingaTimestamp { sec: 0, us: 0 }
    }
}

impl From<f64> for IcingaTimestamp {
    fn from(ts: f64) -> Self {
        let sec = ts as u64;
        let us = (ts - sec as f64) * MICRO_SECONDS_PER_SECOND;
        IcingaTimestamp { sec, us: us as u32 }
    }
}

impl From<IcingaTimestamp> for f64 {
    fn from(ts: IcingaTimestamp) -> Self {
        ts.sec as f64 + ts.us as f64 / MICRO_SECONDS_PER_SECOND
    }
}

impl From<&IcingaTimestamp> for f64 {
    fn from(ts: &IcingaTimestamp) -> Self {
        ts.sec as f64 + ts.us as f64 / MICRO_SECONDS_PER_SECOND
    }
}

impl Display for IcingaTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:.06}", f64::from(self)))
    }
}

impl Default for IcingaTimestamp {
    fn default() -> Self {
        UNIX_EPOCH
            .elapsed()
            .expect("Collect your nobel price for time travel")
            .as_secs_f64()
            .into()
    }
}

impl Serialize for IcingaTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.into())
    }
}

impl<'de> Deserialize<'de> for IcingaTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_f64(TimeStampVisitor)
    }
}

struct TimeStampVisitor;

impl<'de> Visitor<'de> for TimeStampVisitor {
    type Value = IcingaTimestamp;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("Expected floating point number as timestamp")
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.into())
    }
}
