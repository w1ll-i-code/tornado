use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatefulExecutor, StatelessExecutor};
use tornado_common_api::metrics::{ACTION_ID_LABEL_KEY, ACTION_RESULT_KEY, ACTION_RESULT_SUCCESS};

pub mod callback;
pub mod pool;
pub mod retry;

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait(?Send)]
pub trait Command<Message, Output> {
    async fn execute(&self, message: Message) -> Output;
}

/// Implement the Command pattern for StatelessExecutor
#[async_trait::async_trait(?Send)]
impl<T: StatelessExecutor> Command<Arc<Action>, Result<(), ExecutorError>> for T {
    async fn execute(&self, message: Arc<Action>) -> Result<(), ExecutorError> {
        let result = (self as &T).execute(message.clone()).await;
        if result.is_err() {
            (self as &T).get_action_meter().action_failed_retries_counter.add(1, &[
                ACTION_ID_LABEL_KEY.string(message.id.to_owned()),
                ACTION_RESULT_KEY.string(ACTION_RESULT_SUCCESS)
            ]);
        };
        result
    }
}

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait(?Send)]
pub trait CommandMut<Message, Output> {
    async fn execute(&mut self, message: Message) -> Output;
}

/// Implement the Command pattern for StatefulExecutor
#[async_trait::async_trait(?Send)]
impl<T: StatefulExecutor> CommandMut<Arc<Action>, Result<(), ExecutorError>> for T {
    async fn execute(&mut self, message: Arc<Action>) -> Result<(), ExecutorError> {
        (self as &mut T).execute(message).await
    }
}

pub struct CommandMutWrapper<Message, Output, C: CommandMut<Message, Output>> {
    command: Rc<RefCell<C>>,
    phantom_message: PhantomData<Message>,
    phantom_output: PhantomData<Output>,
}

impl<Message, Output, C: CommandMut<Message, Output>> CommandMutWrapper<Message, Output, C> {
    pub fn new(command: Rc<RefCell<C>>) -> Self {
        Self { command, phantom_message: PhantomData, phantom_output: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<I, O, T: CommandMut<I, O>> Command<I, O> for CommandMutWrapper<I, O, T> {
    async fn execute(&self, message: I) -> O {
        let mut command = self.command.borrow_mut();
        command.execute(message).await
    }
}
