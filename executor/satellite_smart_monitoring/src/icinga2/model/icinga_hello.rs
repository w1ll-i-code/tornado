use bitfield_struct::bitfield;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};

/// Icinga sends with it's `icinga::Hello` message currently two parameters:
///
///     * The version of the client, encoded as e.g. 21300 for v2.13.0.
///
///     * The permission of the client, a bitfield of size 64, encoded as a double
///
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IcingaHelloParams {
    pub capabilities: Capabilities,
    pub version: u32,
}

/// Right now only one permission is implemented, however this bitfield is future-prove if ever
/// there is more added. Furthermore it allows for easier deserialization. The current permissions
/// are:
///
///     * execute_arbitrary_command = 1u
///
#[bitfield(u64)]
#[derive(PartialEq)]
pub struct Capabilities {
    pub execute_arbitrary_command: bool,
    #[bits(63)]
    _padding: u64,
}

impl Serialize for Capabilities {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(u64::from(*self))
    }
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u64(PermissionsVisitor)
    }
}

struct PermissionsVisitor;

impl Visitor<'_> for PermissionsVisitor {
    type Value = Capabilities;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("Expected Capabilities as a 64 bit Bitfield encoded as an f64.")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.into())
    }
}

impl IcingaHelloParams {
    pub fn get_version(&self) -> u64 {
        self.version as u64
    }

    pub fn get_permissions(&self) -> Capabilities {
        self.capabilities
    }
}

impl Display for Capabilities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.execute_arbitrary_command() {
            f.write_str("execute_arbitrary_command")
        } else {
            f.write_str("none")
        }
    }
}

impl Display for IcingaHelloParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let major = self.get_version() / 10_000;
        let minor = self.get_version() % 10_000 / 100;
        let bugfix = self.get_version() % 100;

        f.write_fmt(format_args!(
            "Version: {}.{}.{}, Permissions: {}",
            major, minor, bugfix, self.capabilities
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::icinga2::model::{IcingaMethods, JsonRpc};

    #[test]
    pub fn should_deserialize() {
        let msg = r#"{"jsonrpc":"2.0","method":"icinga::Hello","params":{"capabilities":1,"version":21307}}"#;

        let rpc: JsonRpc = serde_json::from_str(msg).unwrap();
        let params = match rpc.method {
            IcingaMethods::Hello(params) => params,
            _ => panic!("Unexpected method."),
        };

        assert!(params.capabilities.execute_arbitrary_command());
        assert_eq!(params.version, 21307);
    }
}
