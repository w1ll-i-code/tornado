use clap::Parser;
use log::trace;
use rand::distributions::WeightedIndex;
use rand::{thread_rng, Rng};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tornado_common::metrics::ActionMeter;
use tornado_common_api::{Action, Payload};
use tornado_executor_common::{StatefulExecutor, StatelessExecutor};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_parallel_smart_monitoring::ParallelSmartMonitoringExecutor;
use tornado_executor_smart_monitoring_check_result::{
    SimpleCreateAndProcess, SmartMonitoringExecutor,
};

#[derive(Parser, Debug)]
struct Params {
    #[arg(short, long)]
    host: usize,
    #[arg(short, long)]
    service: usize,
    #[arg(short, long)]
    weights: Option<Vec<u32>>,
}

impl Params {
    fn get_random(&self) -> Option<(&[u32], Vec<usize>)> {
        match self.weights.as_deref() {
            None => None,
            Some(weights) => {
                let choices = (0..weights.len()).collect();
                Some((weights, choices))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let params = Params::parse();

    let icinga_config = Icinga2ClientConfig {
        server_api_url: "icinga-master".to_string(),
        username: "tornado".to_string(),
        password: "tornado".to_string(),
        disable_ssl_verification: true,
        timeout_secs: None,
    };

    let director_config = DirectorClientConfig {
        server_api_url: "icinga-master".to_string(),
        username: "tornado".to_string(),
        password: "tornado".to_string(),
        disable_ssl_verification: true,
        timeout_secs: None,
    };

    let executor = SmartMonitoringExecutor::new(icinga_config, director_config).unwrap();
    let mut executor = ParallelSmartMonitoringExecutor::new(
        100,
        executor,
        Arc::new(ActionMeter::new("pippo")),
        Default::default(),
    );

    let hosts = params.host;
    let services_per_host = params.service;

    let requests = if let Some((weights, choices)) = params.get_random() {
        generate_requests_random(hosts, services_per_host, weights, &choices)
    } else {
        generate_requests(hosts, services_per_host)
    };

    *tornado_executor_parallel_smart_monitoring::MEASUREMENTS.lock().await =
        Vec::with_capacity(hosts * (services_per_host + 1));

    tokio::time::sleep(Duration::from_secs(5)).await;

    for request in requests {
        let action = Arc::new(Action::new_with_payload_and_created_ms(
            "parallel-monitoring-executor",
            to_payload(request),
            UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
        ));

        executor.execute(action).await.unwrap();
    }

    trace!("Sent all requests");

    loop {
        {
            let measurements =
                tornado_executor_parallel_smart_monitoring::MEASUREMENTS.lock().await;
            if measurements.len() == measurements.capacity() {
                let data = serde_json::to_string(&*measurements).unwrap();
                tokio::fs::write(
                    format!("{hosts}-{services_per_host}-results.json"),
                    data.as_bytes(),
                )
                .await
                .unwrap_or_else(|_| println!("{data}"));
                return;
            }

            trace!("missing measurements: {} != {}", measurements.len(), measurements.capacity());
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

fn to_payload<T: Serialize>(request: T) -> Payload {
    match serde_json::to_value(request) {
        Ok(Value::Object(payload)) => payload,
        _ => unreachable!(),
    }
}

fn generate_requests(hosts: usize, services_per_host: usize) -> Vec<SimpleCreateAndProcess> {
    let mut requests = Vec::with_capacity(hosts * (services_per_host + 1));

    for host_number in 0..hosts {
        let host = json!({
           "object_name": format!("myhost-{host_number:06}"),
           "address": "127.0.0.1",
           "check_command": "hostalive",
           "vars": {
              "location": "Rome"
           }
        });

        requests.push(SimpleCreateAndProcess {
            check_result: to_payload(generate_cr()),
            host: to_payload(host.clone()),
            service: None,
        });

        for service_number in 0..services_per_host {
            let service = json!({
               "object_name": "myservice",
               "check_command": "ping"
            });

            requests.push(SimpleCreateAndProcess {
                check_result: to_payload(generate_cr()),
                host: to_payload(host.clone()),
                service: Some(to_payload(service)),
            })
        }
    }

    requests
}

fn generate_requests_random(
    hosts: usize,
    services_per_host: usize,
    weights: &[u32],
    choices: &[usize],
) -> Vec<SimpleCreateAndProcess> {
    let mut requests = Vec::with_capacity(hosts * (services_per_host + 1));

    let mut dist = WeightedIndex::new(&weights).unwrap();
    let rng = thread_rng();

    for _ in 0..hosts {
        let host_number = choices[dist.sample(&rng)];
        let host = json!({
           "object_name": format!("myhost-{host_number:06}"),
           "address": "127.0.0.1",
           "check_command": "hostalive",
           "vars": {
              "location": "Rome"
           }
        });

        requests.push(SimpleCreateAndProcess {
            check_result: to_payload(generate_cr()),
            host: to_payload(host.clone()),
            service: None,
        });

        for service_number in 0..services_per_host {
            let service = json!({
               "object_name": "myservice",
               "check_command": "ping"
            });

            requests.push(SimpleCreateAndProcess {
                check_result: to_payload(generate_cr()),
                host: to_payload(host.clone()),
                service: Some(to_payload(service)),
            })
        }
    }

    requests
}

fn generate_cr() -> Value {
    json!({
        "exit_status": "2",
        "plugin_output": "Output message"
    })
}
