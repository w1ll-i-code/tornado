mod certification;
mod check_result;
pub mod config;
mod icinga_hello;
mod log_position;
mod timestamp;
mod update_object;
mod verifier;

use serde::{Deserialize, Serialize};

pub use super::model::certification::{
    RequestCertificate, UpdateCertificate, UpdateCertificateResult,
};
pub use super::model::check_result::*;
pub use super::model::icinga_hello::Capabilities;
pub use super::model::icinga_hello::IcingaHelloParams;
pub use super::model::log_position::LogPosition;
pub use super::model::timestamp::IcingaTimestamp;
pub use super::model::update_object::DeleteObject;
pub use super::model::update_object::UpdateObjectParams;
use super::model::verifier::Validator;

/// The messages between Icinga2 are transmitted via the JSON-RPC 2.0 protocol. A JSON-RPC request
/// Object has the following structure:
///
/// ``` json
/// {
///     "jsonrpc": "2.0",
///     "method": "<method name>,
///     "params": { .. }
/// }
/// ```
///
/// Where the JSON-RPC requests from icinga2 differ however from the official specification is the
/// `id` field of the official specification is renamed `ts` for their implementation and contains
/// a timestamp encoded as a floating point number in seconds with precision to the micro seconds.
///
/// Furthermore, icinga2's specification does not make use of any response objects, instead opting
/// to periodically send `log::UpdateLogPosition` requests which may contain a previously sent ts.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JsonRpc {
    #[serde(with = "VersionValidator", rename = "jsonrpc")]
    version: (),
    #[serde(flatten)]
    pub method: IcingaMethods,
    #[serde(skip_serializing_if = "Option::is_none")]
    ts: Option<IcingaTimestamp>,
}

impl From<IcingaMethods> for JsonRpc {
    fn from(method: IcingaMethods) -> Self {
        JsonRpc { version: (), method, ts: None }
    }
}

pub(crate) struct VersionValidator;

impl Validator for VersionValidator {
    const EXPECTED_VALUE: &'static str = "2.0";
}

impl JsonRpc {
    pub fn get_timestamp(&self) -> Option<IcingaTimestamp> {
        self.ts
    }
}

/// All [icinga2 methods](https://icinga.com/docs/icinga-2/latest/doc/19-technical-concepts/#json-rpc-message-api)
/// currently supported. This Type represents the method of the JSON-RPC Message and the tuple
/// parameter contains the `params` of the message. Methods that are implemented and ready for
/// use have a struct as a parameter to ensure valid serialization and deserialization. If a
/// messages can be received but the handling is not yet implemented it has an empty struct to
/// ignore the values passed as parameters as an easy way to ensure the deserialization always
/// succeed, without having any limitation on the contained keys and values. The exeption here is
/// `event::Heartbeat` which doesn't accept any parameters.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "method", content = "params")]
pub enum IcingaMethods {
    /// The `icinga::Hello` message is sent once, as the first message of the connection to establish
    /// a JSON-RPC connection. Since both the Http and JSON-RPC are accepted over the same port.
    /// This tells the endpoint which protocol we are using
    #[serde(rename = "icinga::Hello")]
    Hello(IcingaHelloParams),

    /// The `icinga::Heartbeat` message is sent once every 20 seconds to keep the connection alive and
    /// avoid stale connections. If the master node doesn't receive a message for more than one minute,
    /// it will close the connection.
    #[serde(rename = "event::Heartbeat")]
    HeartBeats {},

    /// An `event::CheckResult` message is used to send CheckResults between the Endpoints. Such a
    /// message has always a host as a parameter, on which the check was performed. If the check was
    /// for a service, the service is also in the parameters. Lastly the params contain a serialized
    /// CheckResult in the Json format.
    #[serde(rename = "event::CheckResult")]
    CheckResult(CheckResultParams),

    /// The `config::Update` message is sent once at the very beginning of the connection by the
    /// master node to synchronize the configuration files.
    #[serde(rename = "config::Update")]
    ConfigUpdate {},

    /// The `config::UpdateObject` message is sent for each known object of the master to
    /// synchronize with the endpoint at the beginning of the connection, after the `config::Update`
    /// and before any other message. After that initial synchronization phase, it is sent, whenever
    /// a new object is added to the cluster.
    ///
    /// If the master accepts configurations, this can also be used to create new objects.
    #[serde(rename = "config::UpdateObject")]
    UpdateObject(UpdateObjectParams),

    /// The `config::DeleteObject` message informs the endpoint that a object has been deleted and
    /// does therefore no longer exist
    #[serde(rename = "config::DeleteObject")]
    DeleteObject(DeleteObject),

    /// The `pki::RequestCertificate` message allows the endpoint to request a new certificate with
    /// the pki ticket generated by the master. This is used by the `icinga node wizard` command and
    /// can probably be used to ease the installation by the end user, by only requiring them to
    /// provide the ticket.
    #[serde(rename = "pki::RequestCertificate")]
    RequestCertificate(RequestCertificate),

    /// If the client requesting a certificate with the `pki::RequestCertificate` does not already
    /// have a valid certificate, the master node will respond with a `pki::UpdateCertificate`
    /// message, which contains the public CA of the master, as well as a certificate which was
    /// signed by the master.
    #[serde(rename = "pki::UpdateCertificate")]
    UpdateCertificate(UpdateCertificate),

    /// The `log::SetLogPosition` message is a response to a `event::CheckResult` message, which had
    /// provided a timestamp _"ts"_ alongside the methode name and the params in the JSON-RPC body. It
    /// confirms that the master node has successfully received the message.
    #[serde(rename = "log::SetLogPosition")]
    SetLogPosition(LogPosition),

    #[serde(rename = "event::SetNextCheck")]
    SetNextCheck {},

    #[serde(rename = "event::SetLastCheckStarted")]
    SetLastCheckStarted {},

    #[serde(rename = "event::SetSuppressedNotifications")]
    SetSuppressedNotifications {},

    #[serde(rename = "event::SetSuppressedNotificationTypes")]
    SetSuppressedNotificationTypes {},

    #[serde(rename = "event::SetNextNotification")]
    SetNextNotification {},

    #[serde(rename = "event::SetForceNextCheck")]
    SetForceNextCheck {},

    #[serde(rename = "event::SetForceNextNotification")]
    SetForceNextNotification {},

    #[serde(rename = "event::SetAcknowledgement")]
    SetAcknowledgement {},

    #[serde(rename = "event::ClearAcknowledgement")]
    ClearAcknowledgement {},

    #[serde(rename = "event::SendNotifications")]
    SendNotifications {},

    #[serde(rename = "event::NotificationSentUser")]
    NotificationSentUser {},

    #[serde(rename = "event::NotificationSentToAllUsers")]
    NotificationSentToAllUsers {},

    #[serde(rename = "event::ExecuteCommand")]
    ExecuteCommand {},

    #[serde(rename = "event::UpdateExecutions")]
    UpdateExecutions {},

    #[serde(rename = "event::ExecutedCommand")]
    ExecutedCommand {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum IcingaObjectType {
    Host,
    Service,
}

#[cfg(test)]
mod test {
    use crate::model::{IcingaMethods, JsonRpc};

    #[test]
    fn jsonrpc_should_serialize_correctly() {
        // Arrange
        let rpc = JsonRpc::from(IcingaMethods::HeartBeats {});

        // Act
        let result = serde_json::to_string(&rpc).unwrap();

        // Assert
        assert_eq!(result, r#"{"jsonrpc":"2.0","method":"event::Heartbeat","params":{}}"#);
    }

    #[test]
    fn jsonrpc_should_deserialize_correctly() {
        // Arrange
        let rpc = r#"{"jsonrpc":"2.0","method":"event::Heartbeat","params":{}}"#;

        // Act
        let result: JsonRpc = serde_json::from_str(rpc).unwrap();

        // Assert
        assert_eq!(result, JsonRpc::from(IcingaMethods::HeartBeats {}));
    }

    #[test]
    fn should_fail_to_deserialize_on_wrong_version() {
        // Arrange
        let rpc = r#"{"jsonrpc":"3.0","method":"event::Heartbeat","params":{}}"#;

        // Act
        let result: Result<JsonRpc, _> = serde_json::from_str(rpc);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_on_missing_version() {
        // Arrange
        let rpc = r#"{"method":"event::Heartbeat","params":{}}"#;

        // Act
        let result: Result<JsonRpc, _> = serde_json::from_str(rpc);

        // Assert
        assert!(result.is_err());
    }
}
