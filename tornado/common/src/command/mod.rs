use crate::metrics::{
    ActionMeter, ACTION_ID_LABEL_KEY, ATTEMPT_RESULT_KEY, RESULT_FAILURE, RESULT_SUCCESS,
};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatefulExecutor, StatelessExecutor};

pub mod callback;
pub mod pool;
pub mod retry;

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait]
pub trait Command<Message, Output>: Send + Sync {
    async fn execute(&self, message: Message) -> Output;
}

#[async_trait::async_trait]
impl<Message, Output, T> Command<Message, Output> for Arc<T>
where
    Message: 'static + Send + Sync,
    T: Command<Message, Output>,
{
    async fn execute(&self, message: Message) -> Output {
        self.execute(message).await
    }
}

pub struct StatelessExecutorCommand<T: StatelessExecutor> {
    action_meter: Arc<ActionMeter>,
    executor: T,
}

impl<T: StatelessExecutor> StatelessExecutorCommand<T> {
    pub fn new(action_meter: Arc<ActionMeter>, executor: T) -> Self {
        Self { action_meter, executor }
    }
}

/// Implement the Command pattern for StatelessExecutorCommand
#[async_trait::async_trait]
impl<T: StatelessExecutor> Command<Arc<Action>, Result<(), ExecutorError>>
    for StatelessExecutorCommand<T>
{
    async fn execute(&self, message: Arc<Action>) -> Result<(), ExecutorError> {
        let action_id = message.id.to_owned();
        let result = self.executor.execute(message).await;
        increment_processing_attempt_counter(&result, action_id, self.action_meter.as_ref());
        result
    }
}

#[inline]
fn increment_processing_attempt_counter<T: Into<Cow<'static, str>>>(
    result: &Result<(), ExecutorError>,
    action_id: T,
    action_meter: &ActionMeter,
) {
    let action_id_label = ACTION_ID_LABEL_KEY.string(action_id);
    match result {
        Ok(_) => action_meter
            .actions_processing_attempts_counter
            .add(1, &[action_id_label, ATTEMPT_RESULT_KEY.string(RESULT_SUCCESS)]),
        Err(_) => action_meter
            .actions_processing_attempts_counter
            .add(1, &[action_id_label, ATTEMPT_RESULT_KEY.string(RESULT_FAILURE)]),
    };
}

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait]
pub trait CommandMut<Message, Output>: Send + Sync {
    async fn execute(&mut self, message: Message) -> Output;
}

pub struct StatefulExecutorCommand<T: StatefulExecutor> {
    action_meter: Arc<ActionMeter>,
    executor: T,
}

impl<T: StatefulExecutor> StatefulExecutorCommand<T> {
    pub fn new(action_meter: Arc<ActionMeter>, executor: T) -> Self {
        Self { action_meter, executor }
    }
}

/// Implement the Command pattern for StatefulExecutorCommand
#[async_trait::async_trait]
impl<T: StatefulExecutor> CommandMut<Arc<Action>, Result<(), ExecutorError>>
    for StatefulExecutorCommand<T>
{
    async fn execute(&mut self, message: Arc<Action>) -> Result<(), ExecutorError> {
        let action_id = message.id.to_owned();
        let result = self.executor.execute(message).await;
        increment_processing_attempt_counter(&result, action_id, self.action_meter.as_ref());
        result
    }
}

pub struct CommandMutWrapper<Message, Output, C: CommandMut<Message, Output>> {
    command: Mutex<C>,
    phantom_message: PhantomData<Message>,
    phantom_output: PhantomData<Output>,
}

impl<Message, Output, C: CommandMut<Message, Output>> CommandMutWrapper<Message, Output, C> {
    pub fn new(command: C) -> Self {
        Self {
            command: Mutex::new(command),
            phantom_message: PhantomData,
            phantom_output: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<I, O, T> Command<I, O> for CommandMutWrapper<I, O, T>
where
    I: Send + Sync,
    O: Send + Sync,
    T: CommandMut<I, O>,
{
    async fn execute(&self, message: I) -> O {
        let mut command = self.command.lock().await;
        command.execute(message).await
    }
}
