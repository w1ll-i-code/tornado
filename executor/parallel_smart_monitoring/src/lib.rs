pub mod config;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use futures_lite::future::FutureExt;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tokio::task::{JoinError, JoinHandle};
use tornado_common::command::pool::CommandPool;
use tornado_common::command::retry::{RetryCommand, RetryStrategy};
use tornado_common::command::{Command, StatelessExecutorCommand};
use tornado_common::metrics::ActionMeter;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatefulExecutor};
use tornado_executor_smart_monitoring_check_result::{
    SimpleCreateAndProcess, SmartMonitoringExecutor,
};

#[derive(Eq, PartialEq)]
struct IcingaObjectId {
    host: Option<String>,
    service: Option<String>,
}

struct RunningFuture {
    id: IcingaObjectId,
    future: JoinHandle<Result<(), ExecutorError>>,
}

impl Future for RunningFuture {
    type Output = Result<Result<(), ExecutorError>, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.poll(cx)
    }
}

pub struct ParallelSmartMonitoringExecutor {
    max_parallel_requests: usize,
    running_futures: Mutex<Vec<RunningFuture>>,
    retry_strategy: RetryStrategy,
    action_meter: Arc<ActionMeter>,
    smart_monitoring_executor: Arc<SmartMonitoringExecutor>,
}

impl ParallelSmartMonitoringExecutor {
    pub fn new(
        max_parallel_requests: usize,
        smart_monitoring_executor: SmartMonitoringExecutor,
        action_meter: Arc<ActionMeter>,
        retry_strategy: RetryStrategy,
    ) -> Self {
        Self {
            max_parallel_requests,
            running_futures: Mutex::new(vec![]),
            retry_strategy,
            action_meter,
            smart_monitoring_executor: Arc::new(smart_monitoring_executor),
        }
    }
}

impl Display for ParallelSmartMonitoringExecutor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParallelSmartMonitoringExecutor")
    }
}

#[async_trait::async_trait]
impl StatefulExecutor for ParallelSmartMonitoringExecutor {
    async fn execute(&mut self, action: Arc<Action>) -> Result<(), ExecutorError> {
        let monitoring_action = SimpleCreateAndProcess::new(&action)?;
        let host_name = monitoring_action.get_host_name().map(|val| val.to_owned());
        let service_name = monitoring_action.get_service_name().map(|val| val.to_owned());

        let id = IcingaObjectId { host: host_name.clone(), service: service_name.clone() };
        let mut running_futures = self.running_futures.lock().await;

        match running_futures.iter().position(|f| f.id == id) {
            None if running_futures.len() == self.max_parallel_requests => {
                let mut futures: FuturesUnordered<_> = running_futures.iter_mut().collect();
                match futures.next().await {
                    None => unreachable!(),
                    Some(_) => {
                        // Todo: Log potential errors
                    }
                }
            }
            Some(index) => {
                // Todo: Log potential errors
                let _ = running_futures.remove(index).await;
            }
            _ => {}
        }

        running_futures.retain(|f| !f.future.is_finished());
        let future = tokio::spawn(self.new_command(action));
        running_futures.push(RunningFuture { id, future });
        Ok(())
    }
}

impl Clone for ParallelSmartMonitoringExecutor {
    fn clone(&self) -> Self {
        Self {
            max_parallel_requests: self.max_parallel_requests,
            running_futures: Default::default(),
            retry_strategy: self.retry_strategy.clone(),
            action_meter: self.action_meter.clone(),
            smart_monitoring_executor: self.smart_monitoring_executor.clone(),
        }
    }
}

impl ParallelSmartMonitoringExecutor {
    fn new_command(
        &self,
        action: Arc<Action>,
    ) -> impl Future<Output = Result<(), ExecutorError>> + Send {
        let retry_strategy = self.retry_strategy.clone();
        let action_meter = self.action_meter.clone();
        let executor = self.smart_monitoring_executor.clone();

        async {
            let command = RetryCommand::new(
                retry_strategy,
                CommandPool::new(1, StatelessExecutorCommand::new(action_meter, executor)),
            );
            command.execute(action).await
        }
    }
}
