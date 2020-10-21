use action::MonitoringAction;
use log::*;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError, RetriableError};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_director::{
    DirectorAction, DirectorExecutor, ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE,
};
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_icinga2::{Icinga2Executor, ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE};

pub const MONITORING_ACTION_NAME_KEY: &str = "action_name";

mod action;

/// An executor that performs a process check result and, if needed, creates the underneath host/service
pub struct MonitoringExecutor {
    icinga_executor: Icinga2Executor,
    director_executor: DirectorExecutor,
}

impl std::fmt::Display for MonitoringExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("MonitoringExecutor")?;
        Ok(())
    }
}

impl MonitoringExecutor {
    pub fn new(
        icinga2_client_config: Icinga2ClientConfig,
        director_client_config: DirectorClientConfig,
    ) -> Result<MonitoringExecutor, ExecutorError> {
        Ok(MonitoringExecutor {
            icinga_executor: Icinga2Executor::new(icinga2_client_config)?,
            director_executor: DirectorExecutor::new(director_client_config)?,
        })
    }

    fn perform_creation_of_icinga_objects(
        &self,
        director_host_creation_action: DirectorAction,
        director_service_creation_action: Option<DirectorAction>,
    ) -> Result<(), ExecutorError> {
        let host_creation_result =
            self.director_executor.perform_request(director_host_creation_action);
        match host_creation_result {
            Ok(()) => {
                debug!("MonitoringExecutor - Director host creation action successfully performed");
                Ok(())
            }
            Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                if code.eq(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) =>
            {
                debug!("MonitoringExecutor - Director host creation action failed with message {:?}. Looks like the host already exists in Icinga.", message);
                Ok(())
            }
            Err(err) => {
                error!(
                    "MonitoringExecutor - Director host creation action failed with error {:?}.",
                    err
                );
                Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error during the host creation. DirectorExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None })
            }
        }?;

        if let Some(director_service_creation_action) = director_service_creation_action {
            let service_creation_result =
                self.director_executor.perform_request(director_service_creation_action);
            match service_creation_result {
                Ok(()) => {
                    debug!("MonitoringExecutor - Director service creation action successfully performed");
                    Ok(())
                }
                Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                    if code.eq(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) =>
                {
                    debug!("MonitoringExecutor - Director service creation action failed with message {:?}. Looks like the host already exists in Icinga.", message);
                    Ok(())
                }
                Err(err) => {
                    error!("MonitoringExecutor - Director service creation action failed with error {:?}.", err);
                    Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error during the service creation. DirectorExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None })
                }
            }?;
        };
        Ok(())
    }
}

impl Executor for MonitoringExecutor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        trace!("MonitoringExecutor - received action: \n[{:?}]", action);

        let mut monitoring_action = MonitoringAction::new(&action)?;

        let (icinga2_action, director_host_creation_action, director_service_creation_action) =
            monitoring_action.to_sub_actions()?;

        let icinga2_action_result = self.icinga_executor.perform_request(&icinga2_action);

        match icinga2_action_result {
            Ok(_) => {
                debug!("MonitoringExecutor - Process check result correctly performed");
                Ok(())
            }
            Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                if code.eq(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE) =>
            {
                debug!("MonitoringExecutor - Process check result action failed with message {:?}. Looks like Icinga2 object does not exist yet. Proceeding with the creation of the object..", message);
                self.perform_creation_of_icinga_objects(
                    director_host_creation_action,
                    director_service_creation_action,
                )?;
                self.icinga_executor.perform_request(&icinga2_action).map_err(|err| ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error while performing the process check result after the object creation. IcingaExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None })
            }
            Err(err) => {
                error!(
                    "MonitoringExecutor - Process check result action failed with error {:?}.",
                    err
                );
                Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error while performing the process check result. IcingaExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::{Mock, MockServer};
    use maplit::*;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange
        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let action = Action::new("");

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::ConfigurationError {
                message:
                    "Invalid Monitoring Action configuration. Err: missing field `action_name`"
                        .to_owned()
            }),
            result
        );
    }

    #[test]
    fn should_throw_error_if_action_name_is_not_valid() {
        // Arrange
        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action
            .payload
            .insert("action_name".to_owned(), Value::Text("my_invalid_action".to_owned()));
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "host".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(&action);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("unknown variant `my_invalid_action`"))
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_throw_error_if_service_action_but_service_creation_payload_not_given() {
        // Arrange
        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_service_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "service".to_owned() => Value::Text("myservice".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::ConfigurationError {
                message: "Invalid Monitoring Action configuration. Err: missing field `service_creation_payload`".to_owned()
            }),
            result
        );
    }

    #[test]
    fn should_return_ok_if_action_name_is_valid() {
        // Arrange
        let mock_server = MockServer::start();

        Mock::new()
            .expect_method(POST)
            .expect_path("/process-check-result")
            .return_status(200)
            .create_on(&mock_server);

        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: mock_server.url(""),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "host".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(&action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_throw_error_if_process_check_result_host_not_specified_with_host_field() {
        // Arrange
        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "filter".to_owned() => Value::Text("host.name==\"myhost\"".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(&action);

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Monitoring action expects that Icinga objects affected by the action are specified with field 'host'"))
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_throw_error_if_process_check_result_service_not_specified_with_service_field() {
        // Arrange
        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_service_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "filter".to_owned() => Value::Text("host.name==\"myhost\"".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(&action);

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Monitoring action expects that Icinga objects affected by the action are specified with field 'service'"))
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_return_ok_if_action_type_is_host_and_service_creation_payload_not_given() {
        // Arrange
        let mock_server = MockServer::start();

        Mock::new()
            .expect_method(POST)
            .expect_path("/process-check-result")
            .return_status(200)
            .create_on(&mock_server);

        let mut executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: mock_server.url(""),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "host".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(&action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }
}
