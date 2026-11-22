use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::models::{ConnectionStatus, DiagnosticLog, ServerNode, Subscription, TrafficStats};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum GhostEvent {
    StateChanged {
        status: ConnectionStatus,
        node: Option<ServerNode>,
        message: Option<String>,
    },
    TrafficUpdated {
        stats: TrafficStats,
    },
    LogAppended {
        log: DiagnosticLog,
    },
    NodesUpdated {
        nodes: Vec<ServerNode>,
    },
    SubscriptionsUpdated {
        subscriptions: Vec<Subscription>,
    },
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<GhostEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: GhostEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<GhostEvent> {
        self.sender.subscribe()
    }
}
