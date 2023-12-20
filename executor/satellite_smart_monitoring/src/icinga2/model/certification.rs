use serde::{Deserialize, Serialize};

/// To get a secure connection to the Icinga2 master instance, a signed certificate is needed.
/// This has two fields:
///
/// - ticket: A generated ticket which represents the requested common name.
///   This can be generated with the `icinga2-master pki ticket --cn icinga2-tornado-satellite`
///   command.
///
/// - cert_request: An optional certification request to be signed. This needs to be present
///   for the first request only.
///
/// It is expected that the icinga2 master will then return a `pki::UpdateCertificate` request.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RequestCertificate {
    pub ticket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_request: Option<String>,
}

/// This message is sent by icinga2 as a response to the `pki::RequestCertificate` request.
/// It can either return with a status_code of `0.0` and `UpdateCertificateResult::Ok{..}` or
/// status_code of `1.0` and `UpdateCertificateResult::Error{..}`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateCertificate {
    pub status_code: f64,
    #[serde(flatten)]
    pub result: UpdateCertificateResult,
}

/// The content of the `UpdateCertificate` response, depending on the status_code.
///
/// **WARNING:**  Is the exposed certificate already valid e up-to-date this will return an error
/// with text `"The certificate for CN '" + cn + "' is valid and uptodate. Skipping automated renewal."`
/// and status-code 1.0. The same status code as if it did not work to sign the request.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum UpdateCertificateResult {
    Error { error: String },
    Ok { cert: String, ca: String, fingerprint_request: String },
}

#[cfg(test)]
mod test {
    use super::RequestCertificate;
    use crate::icinga2::model::{IcingaMethods, JsonRpc};

    #[test]
    fn should_serialize_request_certificate_correctly() {
        let request = JsonRpc::from(IcingaMethods::RequestCertificate(RequestCertificate {
            ticket: "fbd6023742418f57c69b7f6e5e4c7f81c9fbc9a2".to_string(),
            cert_request: None,
        }));

        let expected = r#"{"jsonrpc":"2.0","method":"pki::RequestCertificate","params":{"ticket":"fbd6023742418f57c69b7f6e5e4c7f81c9fbc9a2"}}"#;

        let result = serde_json::to_string(&request).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn should_deserialize_request_certificate_correctly() {
        let expected = JsonRpc::from(IcingaMethods::RequestCertificate(RequestCertificate {
            ticket: "fbd6023742418f57c69b7f6e5e4c7f81c9fbc9a2".to_string(),
            cert_request: None,
        }));

        let request = r#"{"jsonrpc":"2.0","method":"pki::RequestCertificate","params":{"ticket":"fbd6023742418f57c69b7f6e5e4c7f81c9fbc9a2"}}"#;

        let result = serde_json::from_str::<JsonRpc>(request).unwrap();

        assert_eq!(result, expected);
    }
}
