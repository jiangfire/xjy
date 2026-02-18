use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

pub type WsSender = mpsc::UnboundedSender<String>;

#[derive(Clone)]
pub struct NotificationHub {
    connections: Arc<DashMap<i32, Vec<WsSender>>>,
}

impl Default for NotificationHub {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationHub {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    pub fn subscribe(&self, user_id: i32) -> mpsc::UnboundedReceiver<String> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections.entry(user_id).or_default().push(tx);
        rx
    }

    pub fn send_to_user(&self, user_id: i32, message: &str) {
        if let Some(mut senders) = self.connections.get_mut(&user_id) {
            // Remove closed channels while sending
            senders.retain(|sender| sender.send(message.to_string()).is_ok());
            if senders.is_empty() {
                drop(senders);
                self.connections.remove(&user_id);
            }
        }
    }
}
