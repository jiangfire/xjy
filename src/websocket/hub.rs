use dashmap::DashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::mpsc;

pub type WsSender = mpsc::UnboundedSender<String>;

#[derive(Clone)]
pub struct NotificationHub {
    connections: Arc<DashMap<i32, Vec<(u64, WsSender)>>>,
    next_conn_id: Arc<AtomicU64>,
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
            next_conn_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn subscribe(&self, user_id: i32) -> (u64, mpsc::UnboundedReceiver<String>) {
        let conn_id = self.next_conn_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections
            .entry(user_id)
            .or_default()
            .push((conn_id, tx));
        (conn_id, rx)
    }

    pub fn unsubscribe(&self, user_id: i32, conn_id: u64) {
        if let Some(mut senders) = self.connections.get_mut(&user_id) {
            senders.retain(|(id, _)| *id != conn_id);
            if senders.is_empty() {
                drop(senders);
                self.connections.remove(&user_id);
            }
        }
    }

    pub fn send_to_user(&self, user_id: i32, message: &str) {
        if let Some(mut senders) = self.connections.get_mut(&user_id) {
            // Remove closed channels while sending
            senders.retain(|(_, sender)| sender.send(message.to_string()).is_ok());
            if senders.is_empty() {
                drop(senders);
                self.connections.remove(&user_id);
            }
        }
    }
}
