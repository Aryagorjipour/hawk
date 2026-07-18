use std::sync::Arc;

use parking_lot::RwLock;
use tracing::{error, info};

use crate::domain::DomainEvent;

pub type EventHandler = Arc<dyn Fn(&DomainEvent) + Send + Sync>;

#[derive(Default, Clone)]
pub struct EventBus {
    handlers: Arc<RwLock<Vec<EventHandler>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, handler: EventHandler) {
        self.handlers.write().push(handler);
    }

    pub fn publish(&self, event: DomainEvent) {
        info!(event = event.name(), "domain_event");
        for h in self.handlers.read().iter() {
            // Handlers must not panic; isolate with catch_unwind at call site if needed.
            h(&event);
        }
    }
}

pub fn tracing_handler() -> EventHandler {
    Arc::new(|event: &DomainEvent| {
        info!(event = ?event, "domain_event_detail");
    })
}

pub fn safe_publish(bus: &EventBus, event: DomainEvent) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bus.publish(event)));
    if let Err(e) = result {
        error!(?e, "event handler panicked");
    }
}
