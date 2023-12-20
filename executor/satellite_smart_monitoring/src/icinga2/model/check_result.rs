use crate::icinga2::model::verifier::Validator;
use crate::icinga2::model::IcingaTimestamp;
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use std::fmt::Formatter;

/// The `event:CheckResult` message can either be sent for a host or a service. Depending on the
/// object, different values are expected:
///
/// A host has only the hostname and the CheckResult while the Service has an additional field
/// for the service name. Additionally, a host can only have one of two states: "UP" or "DOWN",
/// while services have four states: "OK", "WARNING", "ERROR" and "UNKNOWN"
///
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum CheckResultParams {
    Host { host: String, cr: HostCheckResult },
    Service { host: String, service: String, cr: ServiceCheckResult },
}

impl CheckResultParams {
    pub fn name(&self) -> String {
        match self {
            CheckResultParams::Host { host, .. } => host.clone(),
            CheckResultParams::Service { host, service, .. } => format!("{}!{}", host, service),
        }
    }

    #[allow(dead_code)]
    pub fn for_host(host: String, cr: HostCheckResult) -> CheckResultParams {
        Self::Host { host, cr }
    }

    #[allow(dead_code)]
    pub fn for_service(host: String, service: String, cr: ServiceCheckResult) -> CheckResultParams {
        Self::Service { host, service, cr }
    }
}

/// The HostCheckResult has a [HostState](model::check_result::HostState) and a bunch of optional data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HostCheckResult {
    pub state: HostState,
    #[serde(flatten)]
    pub cr: Box<SharedCheckResult>,
}

/// The state of a host. A host can either be "UP" or "DOWN"
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum HostState {
    UP = 0,
    DOWN = 1,
}

impl Serialize for HostState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f32((*self as u32) as f32)
    }
}

impl<'de> Deserialize<'de> for HostState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(HostStateVisitor)
    }
}

struct HostStateVisitor;

impl<'de> Visitor<'de> for HostStateVisitor {
    type Value = HostState;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(
            "a HostState, either one of [\"UP\", \"DOWN\"] or their integer representations [0, 1]",
        )
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match v {
            0 => Ok(HostState::UP),
            _ => Ok(HostState::DOWN),
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v > u64::MAX as f64 {
            Err(E::invalid_value(
                Unexpected::Float(v),
                &"WTF are you sending me? This state is way to high. What are you even doing? Trying to crash the service with an overflow? Well, not today bitch."
            ))
        } else {
            self.visit_u64(v as u64)
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.eq_ignore_ascii_case("UP") {
            Ok(HostState::UP)
        } else if v.eq_ignore_ascii_case("DOWN") {
            Ok(HostState::DOWN)
        } else {
            Err(E::invalid_value(Unexpected::Str(v), &"one of [\"UP\", \"DOWN\"]"))
        }
    }
}

/// The ServiceCheckResult has a [ServiceState](model::check_result::ServiceState) and a bunch of optional data.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ServiceCheckResult {
    pub state: ServiceState,
    #[serde(flatten)]
    pub cr: Box<SharedCheckResult>,
}

/// The state of a service.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum ServiceState {
    OK = 0,
    WARNING = 1,
    ERROR = 2,
    UNKNOWN = 3,
}

impl Serialize for ServiceState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f32((*self as u32) as f32)
    }
}

impl<'de> Deserialize<'de> for ServiceState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ServiceStateVisitor)
    }
}

struct ServiceStateVisitor;

impl<'de> Visitor<'de> for ServiceStateVisitor {
    type Value = ServiceState;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(r#"a ServiceState, either one of ["OK", "WARNING", "ERROR", "Unknown"] or their integer representations [0, 1, 2, 3]"#)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match v {
            0 => Ok(ServiceState::OK),
            1 => Ok(ServiceState::WARNING),
            2 => Ok(ServiceState::ERROR),
            _ => Ok(ServiceState::UNKNOWN),
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v > u64::MAX as f64 {
            Err(E::invalid_value(
                Unexpected::Float(v),
                &"WTF are you sending me? This state is way to high. What are you even doing? Trying to crash the service with an overflow? Well, not today bitch."
            ))
        } else {
            self.visit_u64(v as u64)
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.eq_ignore_ascii_case("OK") {
            Ok(ServiceState::OK)
        } else if v.eq_ignore_ascii_case("WARNING") {
            Ok(ServiceState::WARNING)
        } else if v.eq_ignore_ascii_case("ERROR") {
            Ok(ServiceState::ERROR)
        } else if v.eq_ignore_ascii_case("UNKNOWN") {
            Ok(ServiceState::UNKNOWN)
        } else {
            Err(E::invalid_value(
                Unexpected::Str(v),
                &r#"one of ["Ok", "WARNING", "ERROR", "UNKNOWN"]"#,
            ))
        }
    }
}

/// The CheckResult is the core concept of the icinga2 protocol. It is sent to update the master
/// about the state of a certain checkable object. The CheckResult has to have three values in order
/// to be correctly processed by the master:
///
///     * The type of the CheckResult for the purposes of a satellite which sends passive CheckResults,
///     is always "CheckResult".
///
///     * The state is the core part of the message and needs to be a integer between 1 and 4. However
///     because of the Json specification which has no Integer defined, it is usually represented by
///     icinga2 as a floating point number, with a zero as mantissa.
///
///     * The last object that needs to be present is the performance data. It is a list of Strings
///     which holds data to the check. This value can be empty ([]) as long as the key is present.
///
/// Furthermore it can hold a bunch of other values to provide further information about the test.
/// Those are however all optional.
///
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub struct SharedCheckResult {
    #[serde(rename = "type", default, with = "CheckResultTypeValidator")]
    object_type: (),
    pub performance_data: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_start: Option<IcingaTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_end: Option<IcingaTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_start: Option<IcingaTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_end: Option<IcingaTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vars_before: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vars_after: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<f64>,
}

struct CheckResultTypeValidator;
impl Validator for CheckResultTypeValidator {
    const EXPECTED_VALUE: &'static str = "CheckResult";
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Command {
    String(String),
    Vec(Vec<String>),
}

impl From<String> for Command {
    fn from(command: String) -> Self {
        Command::String(command)
    }
}

impl From<Vec<String>> for Command {
    fn from(command: Vec<String>) -> Self {
        Command::Vec(command)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::check_result::{
        Command, HostCheckResult, HostState, ServiceCheckResult, ServiceState,
    };
    use crate::model::CheckResultParams;

    #[test]
    fn should_serialize_host() {
        // Arrange
        let check_result = CheckResultParams::Host {
            host: "my_host".to_owned(),
            cr: HostCheckResult { state: HostState::UP, cr: Box::default() },
        };

        // Act
        let result = serde_json::to_string(&check_result).unwrap();

        // Assert
        assert_eq!(
            result,
            r#"{"host":"my_host","cr":{"state":0.0,"type":"CheckResult","performance_data":[]}}"#
        )
    }

    #[test]
    fn should_serialize_service() {
        // Arrange
        let check_result = CheckResultParams::Service {
            host: "my_host".to_owned(),
            service: "my_service".to_owned(),
            cr: ServiceCheckResult { state: ServiceState::OK, cr: Box::default() },
        };

        // Act
        let result = serde_json::to_string(&check_result).unwrap();

        // Assert
        assert_eq!(
            result,
            r#"{"host":"my_host","service":"my_service","cr":{"state":0.0,"type":"CheckResult","performance_data":[]}}"#
        )
    }

    #[test]
    fn should_serialize_command_correctly() {
        let msg = Command::String("ls".to_owned());
        let msg2 = Command::Vec(vec!["ls".to_owned()]);

        let command = serde_json::to_string(&msg).unwrap();
        let command2 = serde_json::to_string(&msg2).unwrap();

        assert_eq!(command, r#""ls""#.to_owned());
        assert_eq!(command2, r#"["ls"]"#);
    }

    #[test]
    fn should_deserialize_command_correctly() {
        let msg = r#""ls""#;
        let msg2 = r#"["ls"]"#;

        let command: Command = serde_json::from_str(msg).unwrap();
        let command2: Command = serde_json::from_str(msg2).unwrap();

        assert_eq!(command, Command::String("ls".to_owned()));
        assert_eq!(command2, Command::Vec(vec!["ls".to_owned()]));
    }
}
