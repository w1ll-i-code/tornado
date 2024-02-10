use crate::icinga2::model::config::{Host, Service};
use crate::icinga2::model::{
    CheckResultParams, Command, HostCheckResult, HostState, ServiceCheckResult, ServiceState,
    SharedCheckResult,
};
use crate::satellite::SetStateRequest;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tornado_common_api::{Action, Payload};
use tornado_executor_common::StatelessExecutor;
use tornado_executor_satellite_smart_monitoring::config::{
    EndPointConfig, SatelliteSmartMonitoringConfig,
};
use tornado_executor_satellite_smart_monitoring::SatelliteSmartMonitoringExecutor;

mod config;
mod error;
mod icinga2;
mod satellite;
#[tokio::main]
async fn main() {
    env_logger::init();

    let config = SatelliteSmartMonitoringConfig {
        endpoint: EndPointConfig {
            address: "icinga-master".try_into().unwrap(),
            port: 5665,
            host_name: "icinga-rust-satellite".to_string(),
            ticket: "fbd6023742418f57c69b7f6e5e4c7f81c9fbc9a2".to_string(),
        },
        cert_root_path: PathBuf::from("test_data"),
        retry_time: Duration::from_secs(10),
        max_parallel_send: 10,
        intern_buffer_size: 10_000_000,
    };

    let executor = SatelliteSmartMonitoringExecutor::new(config);

    let hosts = 10;
    let services_per_host = 10;

    let requests = generate_requests(hosts, services_per_host);
    *satellite::MEASUREMENTS.lock().await = Vec::with_capacity(hosts * (services_per_host + 1));

    for request in requests {
        let action = Arc::new(Action::new_with_payload_and_created_ms(
            "satellite-monitoring-executor",
            to_payload(request),
            UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
        ));

        executor.execute(action).await.unwrap();
    }

    loop {
        let measurements = satellite::MEASUREMENTS.lock().await;
        if measurements.len() == measurements.capacity() {
            let data = serde_json::to_string(&*measurements).unwrap();
            tokio::fs::write(format!("{hosts}-{services_per_host}-results.json"), data.as_bytes())
                .await
                .unwrap_or_else(|_| println!("{data}"));
            return;
        }

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

fn to_payload(request: SetStateRequest) -> Payload {
    match serde_json::to_value(request) {
        Ok(Value::Object(payload)) => payload,
        _ => unreachable!(),
    }
}

fn generate_requests(hosts: usize, services_per_host: usize) -> Vec<SetStateRequest> {
    let mut requests = Vec::with_capacity(hosts * (services_per_host + 1));

    for host_number in 0..hosts {
        let host = Host {
            name: format!("host-number-{host_number:06}"),
            check_command: Some(Command::String("dummy".to_owned())),
            zone: Some("master".to_string()),
            groups: Some(vec![format!("host-group-{}", host_number % 64)]),
            address: Some(format!("127.0.0.{}", host_number % 255)),
            max_check_attempts: Some((host_number % 5 + 4) as u32),
            check_interval: Some(format!("{}m", 30 - (host_number % 5) * 5)),
            ..Default::default()
        };

        requests.push(SetStateRequest {
            host: host.clone(),
            service: None,
            check_result: generate_cr(host_number, services_per_host),
        });

        for service_number in 0..services_per_host {
            let service = Service {
                name: format!("service-number-{service_number:03}"),
                host: Some(host.name.clone()),
                ..Default::default()
            };

            requests.push(SetStateRequest {
                host: host.clone(),
                service: None,
                check_result: generate_cr_service(host_number, service_number),
            })
        }
    }

    requests
}

fn generate_cr(host_number: usize, service_number: usize) -> CheckResultParams {
    let mut shared_cr = SharedCheckResult::default();
    shared_cr.active = Some(false);
    shared_cr.check_source = Some(format!("host-number-{host_number}"));
    shared_cr.output =
        Some(format!("host-number-{:08x}-number-host", host_number * service_number));
    CheckResultParams::Host {
        host: format!("host-number-{host_number}"),
        cr: HostCheckResult { state: HostState::UP, cr: Box::new(shared_cr) },
    }
}

fn generate_cr_service(host_number: usize, service_number: usize) -> CheckResultParams {
    let mut shared_cr = SharedCheckResult::default();
    shared_cr.active = Some(false);
    shared_cr.check_source = Some(format!("host-number-{host_number}"));
    shared_cr.output =
        Some(format!("host-number-{:08x}-number-host", host_number * service_number));
    CheckResultParams::Service {
        host: format!("host-number-{host_number}"),
        service: format!("service-number-{service_number}"),
        cr: ServiceCheckResult { state: ServiceState::OK, cr: Box::new(shared_cr) },
    }
}
