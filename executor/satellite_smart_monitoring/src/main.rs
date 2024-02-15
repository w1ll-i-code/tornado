use clap::Parser;
use log::trace;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tornado_common_api::{Action, Payload};
use tornado_executor_common::StatelessExecutor;
use tornado_executor_satellite_smart_monitoring::config::{
    EndPointConfig, SatelliteSmartMonitoringConfig,
};
use tornado_executor_satellite_smart_monitoring::icinga2::model::config::{Host, Service};
use tornado_executor_satellite_smart_monitoring::icinga2::model::{
    CheckResultParams, Command, HostCheckResult, HostState, ServiceCheckResult, ServiceState,
    SharedCheckResult,
};
use tornado_executor_satellite_smart_monitoring::satellite::SetStateRequest;
use tornado_executor_satellite_smart_monitoring::SatelliteSmartMonitoringExecutor;

#[derive(Parser, Debug)]
struct Params {
    #[arg(short, long)]
    host: usize,
    #[arg(short, long)]
    service: usize,
    #[arg(short, long)]
    replays: usize,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let params = Params::parse();

    let config = SatelliteSmartMonitoringConfig {
        endpoint: EndPointConfig {
            address: "icinga-master".try_into().unwrap(),
            port: 5665,
            host_name: "icinga-rust-satellite".to_string(),
            ticket: "fbd6023742418f57c69b7f6e5e4c7f81c9fbc9a2".to_string(),
        },
        cert_root_path: PathBuf::from("test_data"),
        retry_time: Duration::from_secs(10),
        max_parallel_send: 100,
        intern_buffer_size: 100_000,
    };

    let executor = SatelliteSmartMonitoringExecutor::new(config);

    let hosts = params.host;
    let services_per_host = params.service;

    let requests = generate_requests(hosts, services_per_host);
    *tornado_executor_satellite_smart_monitoring::satellite::MEASUREMENTS.lock().await =
        Vec::with_capacity(hosts * (services_per_host + 1));

    tokio::time::sleep(Duration::from_secs(5)).await;

    for replay in 0..params.replays {
        for request in requests {
            let action = Arc::new(Action::new_with_payload_and_created_ms(
                "satellite-monitoring-executor",
                to_payload(request),
                UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
            ));

            executor.execute(action).await.unwrap();
        }

        trace!("Sent all requests");

        'wait: loop {
            {
                let measurements =
                    tornado_executor_satellite_smart_monitoring::satellite::MEASUREMENTS
                        .lock()
                        .await;
                if measurements.len() == measurements.capacity() {
                    let data = serde_json::to_string(&*measurements).unwrap();
                    tokio::fs::write(
                        format!("timings-host{hosts:06}-service{services_per_host:03}-replay{replay:03}.json"),
                        data.as_bytes(),
                    )
                    .await
                    .unwrap_or_else(|_| println!("{data}"));
                    break 'wait;
                }

                trace!(
                    "missing measurements: {} != {}",
                    measurements.len(),
                    measurements.capacity()
                );
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
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
            address: Some(format!("127.0.0.{}", host_number % 255)),
            max_check_attempts: Some((host_number % 5 + 4) as u32),
            enable_active_checks: Some(false),
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
                enable_active_checks: Some(false),
                ..Default::default()
            };

            requests.push(SetStateRequest {
                host: host.clone(),
                service: Some(service),
                check_result: generate_cr_service(host_number, service_number),
            })
        }
    }

    requests
}

fn generate_cr(host_number: usize, service_number: usize) -> CheckResultParams {
    let mut shared_cr = SharedCheckResult::default();
    shared_cr.active = Some(false);
    shared_cr.check_source = Some(format!("host-number-{host_number:06}"));
    shared_cr.output =
        Some(format!("host-number-{:08x}-number-host", host_number * service_number));
    CheckResultParams::Host {
        host: format!("host-number-{host_number:06}"),
        cr: HostCheckResult { state: HostState::UP, cr: Box::new(shared_cr) },
    }
}

fn generate_cr_service(host_number: usize, service_number: usize) -> CheckResultParams {
    let mut shared_cr = SharedCheckResult::default();
    shared_cr.active = Some(false);
    shared_cr.check_source = Some(format!("host-number-{host_number:06}"));
    shared_cr.output =
        Some(format!("host-number-{:08x}-number-host", host_number * service_number));
    CheckResultParams::Service {
        host: format!("host-number-{host_number:06}"),
        service: format!("service-number-{service_number:03}"),
        cr: ServiceCheckResult { state: ServiceState::OK, cr: Box::new(shared_cr) },
    }
}
