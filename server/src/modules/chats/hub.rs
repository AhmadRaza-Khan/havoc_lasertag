//! In-memory registry of live WebSocket connections: who is online, and how to reach them.
//!
//! A player may be connected from several devices; they're online while at least one
//! connection is open. This lives in one process, which fits a single-server deployment.
//! Running several server instances would need a shared bus (e.g. Postgres LISTEN/NOTIFY).

use std::{
    collections::HashMap,
    sync::{
        RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::mpsc::{self, Receiver, Sender, error::TrySendError};
use uuid::Uuid;

use super::events::ServerEvent;

/// Events queued per connection before a slow client starts losing them.
const CONNECTION_BUFFER: usize = 256;

pub struct Connection {
    pub id: u64,
    /// Serialized events to write to the socket.
    pub rx: Receiver<String>,
    /// True if this is the player's first open connection (they just came online).
    pub came_online: bool,
}

#[derive(Default)]
pub struct ChatHub {
    connections: RwLock<HashMap<Uuid, HashMap<u64, Sender<String>>>>,
    next_id: AtomicU64,
}

impl ChatHub {
    pub fn register(&self, player: Uuid) -> Connection {
        let (tx, rx) = mpsc::channel(CONNECTION_BUFFER);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut conns = self.connections.write().unwrap_or_else(|e| e.into_inner());
        let player_conns = conns.entry(player).or_default();
        let came_online = player_conns.is_empty();
        player_conns.insert(id, tx);
        Connection {
            id,
            rx,
            came_online,
        }
    }

    /// Returns true if that was the player's last connection (they went offline).
    pub fn unregister(&self, player: Uuid, connection: u64) -> bool {
        let mut conns = self.connections.write().unwrap_or_else(|e| e.into_inner());
        let Some(player_conns) = conns.get_mut(&player) else {
            return false;
        };
        player_conns.remove(&connection);
        if player_conns.is_empty() {
            conns.remove(&player);
            true
        } else {
            false
        }
    }

    pub fn is_online(&self, player: Uuid) -> bool {
        self.read().contains_key(&player)
    }

    pub fn online_players(&self) -> Vec<Uuid> {
        self.read().keys().copied().collect()
    }

    /// To every connection of `player`.
    pub fn send(&self, player: Uuid, event: &ServerEvent) {
        let Some(text) = serialize(event) else { return };
        if let Some(player_conns) = self.read().get(&player) {
            for tx in player_conns.values() {
                deliver(tx, &text);
            }
        }
    }

    /// To one connection only.
    pub fn send_to_connection(&self, player: Uuid, connection: u64, event: &ServerEvent) {
        let Some(text) = serialize(event) else { return };
        if let Some(tx) = self.read().get(&player).and_then(|c| c.get(&connection)) {
            deliver(tx, &text);
        }
    }

    /// To every connected player except `except`.
    pub fn broadcast_except(&self, except: Uuid, event: &ServerEvent) {
        let Some(text) = serialize(event) else { return };
        for (player, player_conns) in self.read().iter() {
            if *player != except {
                for tx in player_conns.values() {
                    deliver(tx, &text);
                }
            }
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, HashMap<Uuid, HashMap<u64, Sender<String>>>> {
        self.connections.read().unwrap_or_else(|e| e.into_inner())
    }
}

fn serialize(event: &ServerEvent) -> Option<String> {
    serde_json::to_string(event)
        .inspect_err(|e| tracing::error!("failed to serialize chat event: {e}"))
        .ok()
}

/// Never blocks: if a client can't keep up, it misses events rather than stalling everyone.
fn deliver(tx: &Sender<String>, text: &str) {
    if let Err(TrySendError::Full(_)) = tx.try_send(text.to_owned()) {
        tracing::warn!("chat connection buffer full; dropping event");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracks_presence_across_devices() {
        let hub = ChatHub::default();
        let p = Uuid::from_u128(1);

        let a = hub.register(p);
        let b = hub.register(p);
        assert!(a.came_online && !b.came_online);
        assert!(hub.is_online(p));

        assert!(!hub.unregister(p, a.id), "still online on the other device");
        assert!(hub.unregister(p, b.id), "last device gone");
        assert!(!hub.is_online(p));
    }

    #[tokio::test]
    async fn routes_events() {
        let hub = ChatHub::default();
        let (p1, p2) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let mut c1 = hub.register(p1);
        let mut c2 = hub.register(p2);

        hub.send(p2, &ServerEvent::Pong);
        assert_eq!(c2.rx.recv().await.unwrap(), r#"{"type":"pong"}"#);
        assert!(c1.rx.try_recv().is_err(), "p1 must not receive p2's events");

        hub.broadcast_except(p1, &ServerEvent::Pong);
        assert!(c2.rx.try_recv().is_ok());
        assert!(c1.rx.try_recv().is_err());
    }
}
