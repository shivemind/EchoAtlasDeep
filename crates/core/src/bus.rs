/// EventBus wraps tokio::sync::broadcast.
/// Multiple subsystems subscribe independently — no producer coupling.
use tokio::sync::broadcast;

use crate::event::AppEvent;

const BUS_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BUS_CAPACITY);
        Self { sender }
    }

    /// Send an event to all subscribers. Returns the number of receivers that got it.
    /// Errors are ignored — if no subscribers are listening, events are dropped.
    pub fn send(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }

    /// Subscribe to all future events. Each subscriber gets its own independent queue.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    /// Clone just the sender side (for subsystems that only produce events).
    pub fn sender(&self) -> broadcast::Sender<AppEvent> {
        self.sender.clone()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
