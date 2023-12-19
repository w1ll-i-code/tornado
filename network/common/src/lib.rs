use tornado_common::actors::message::ActionMessage;

pub trait EventBus: Send + Sync {
    fn publish_action(&self, message: ActionMessage);
}
