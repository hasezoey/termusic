use termusiclib::types::Msg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    Forward(Msg),
}

impl PartialOrd for UserEvent {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        None
    }
}
