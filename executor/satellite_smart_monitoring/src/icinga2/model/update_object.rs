use crate::icinga2::model::IcingaObjectType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

/// The `config::UpdateObject` message is used to create new object in the master. It contains at
/// least the following values:
///
///     * The name of the object is composed like <hostname>[!<service_name>] for checkable objects.
///     * The type of the object is either "Host" or "Service" for the two checkable Objects.
///     * The version is a number and should always be 0.0 to show that is is the first version, as
///       well as guarantee that it won't overwrite any existing objects.
///
/// Further more has the object a package field which should always be 'director' if the icinga
/// director is used to avoid conflicts on the next deploy. However if that is the case, the objects
/// also needs to be persisted in the director, otherwise it will be overwritten by the next deploy.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UpdateObjectParams {
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: IcingaObjectType,
    version: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified_attributes: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package: Option<String>,
}

#[derive(Debug, Error)]
pub enum UpdateObjectParamsCreationError {
    #[error("UpdateObjectParamsCreationError::InvalidName: {0}")]
    InvalidName(String),
}

/// This builder helps creating a new [UpdateObjectParams](crate::model::UpdateObjectParams).
/// Since the configuration needs to be generated before the object can be sent to icinga2.
/// This ensures that all `UpdateObjectParams` sent have the correct format. Otherwise one might
/// be able to create non valid parameters.
pub struct UpdateHostParamsBuilder {
    name: String,
    pub zone: Option<String>,
    pub modified_attributes: Option<Map<String, Value>>,
    pub original_attributes: Option<Vec<String>>,
    pub config: Option<String>,
}

/// This builder helps creating a new [UpdateObjectParams](crate::model::UpdateObjectParams).
/// Since the configuration needs to be generated before the object can be sent to icinga2.
/// This ensures that all `UpdateObjectParams` sent have the correct format. Otherwise one might
/// be able to create non valid parameters.
pub struct UpdateServiceParamsBuilder {
    name: String,
    host: String,
    pub modified_attributes: Option<Map<String, Value>>,
    pub original_attributes: Option<Vec<String>>,
    pub config: Option<String>,
}

impl UpdateObjectParams {
    pub fn create_host(
        host: &str,
    ) -> Result<UpdateHostParamsBuilder, UpdateObjectParamsCreationError> {
        if host.contains('!') {
            return Err(UpdateObjectParamsCreationError::InvalidName(format!(
                "Invalid name for host. Object can not contain an exclamation point: {}",
                host
            )));
        }

        Ok(UpdateHostParamsBuilder {
            name: host.to_string(),
            zone: None,
            modified_attributes: None,
            original_attributes: None,
            config: None,
        })
    }

    pub fn create_service(
        host: &str,
        service: &str,
    ) -> Result<UpdateServiceParamsBuilder, UpdateObjectParamsCreationError> {
        if host.contains('!') {
            return Err(UpdateObjectParamsCreationError::InvalidName(format!(
                "Invalid name for host. Object can not contain an exclamation point: {}",
                host
            )));
        }

        if service.contains('!') {
            return Err(UpdateObjectParamsCreationError::InvalidName(format!(
                "Invalid name for service. Object can not contain an exclamation point: {}",
                host
            )));
        }

        Ok(UpdateServiceParamsBuilder {
            name: format!("{}!{}", host, service),
            host: host.to_string(),
            modified_attributes: None,
            original_attributes: None,
            config: None,
        })
    }
}

impl UpdateServiceParamsBuilder {
    pub fn build(self) -> UpdateObjectParams {
        let config = match self.config {
            None => {
                format!(
                    "object Service \"{}\" {{\n\thost_name = \"{}\"\n\tcheck_command = \"dummy\"\n}}",
                    self.name, self.host
                )
            }
            Some(config) => config,
        };

        UpdateObjectParams {
            name: self.name,
            object_type: IcingaObjectType::Service,
            version: 0.0,
            config: Some(config),
            modified_attributes: self.modified_attributes,
            original_attributes: self.original_attributes,
            package: None,
        }
    }
}

impl UpdateHostParamsBuilder {
    pub fn build(self) -> UpdateObjectParams {
        let config = match self.config {
            None => {
                let zone = self.zone.as_deref().unwrap_or("");
                format!("object Host \"{}\" {{\n\tcheck_command = \"dummy\"\n{}}}", self.name, zone)
            }
            Some(config) => config,
        };

        UpdateObjectParams {
            name: self.name,
            object_type: IcingaObjectType::Host,
            version: 0.0,
            config: Some(config),
            modified_attributes: self.modified_attributes,
            original_attributes: self.original_attributes,
            package: None,
        }
    }
}

/// Request to delete an object at runtime. Does only work on objects in the `_api` package.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeleteObject {
    pub name: String,
    #[serde(rename = "type")]
    object_type: IcingaObjectType,
    version: f64,
}
