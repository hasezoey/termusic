use termusiclib::types::Msg;
use tokio::sync::mpsc::UnboundedReceiver;
use tuirealm::{
    listener::{ListenerResult, PollAsync},
    Event,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    Forward(Msg),
}

impl PartialOrd for UserEvent {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug)]
pub struct PortRxMain(UnboundedReceiver<Msg>);

impl PortRxMain {
    pub fn new(rx_to_main: UnboundedReceiver<Msg>) -> Self {
        Self(rx_to_main)
    }
}

#[tuirealm::async_trait]
impl PollAsync<UserEvent> for PortRxMain {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        match self.0.recv().await {
            Some(ev) => Ok(Some(Event::User(UserEvent::Forward(ev)))),
            None => Ok(None),
        }
    }
}
