use std::{sync::Arc, time::Duration};

use actix_web_lab::sse::{self, ChannelStream, Sse};
use futures_util::future;
use tokio::sync::Mutex;
use tokio::time::interval;
use serde::Serialize;
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum BroadcastEvent {
    Reload,
    Connected
}

impl BroadcastEvent {
    pub fn to_data(&self) -> sse::Data {
        sse::Data::new_json(&self).unwrap()
    }
}


pub struct Broadcaster {
    inner: Mutex<BroadcasterInner>,
}

#[derive(Debug, Clone, Default)]
struct BroadcasterInner {
    clients: Vec<sse::Sender>,
}

impl Broadcaster {
    /// Constructs new broadcaster and spawns ping loop.
    pub fn create() -> Arc<Self> {
        let this = Arc::new(Broadcaster {
            inner: Mutex::new(BroadcasterInner::default()),
        });

        Broadcaster::spawn_ping(Arc::clone(&this));

        this
    }

    /// Pings clients every 10 seconds to see if they are alive and remove them from the broadcast
    /// list if not.
    fn spawn_ping(this: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;
                this.remove_stale_clients().await;
            }
        });
    }

    /// Removes all non-responsive clients from broadcast list.
    async fn remove_stale_clients(&self) {
        let clients = self.inner.lock().await.clients.clone();

        let mut ok_clients = Vec::new();

        for client in clients {
            if client
                .send(sse::Event::Comment("ping".into()))
                .await
                .is_ok()
            {
                ok_clients.push(client.clone());
            }
        }

        self.inner.lock().await.clients = ok_clients;
    }

    /// Registers client with broadcaster, returning an SSE response body.
    pub async fn new_client(&self) -> Sse<ChannelStream> {
        let (tx, rx) = sse::channel(10);

        tx.send(BroadcastEvent::Connected.to_data()).await.unwrap();

        self.inner.lock().await.clients.push(tx);

        rx
    }

    /// Broadcasts `msg` to all clients.
    pub async fn broadcast(&self, msg: BroadcastEvent) {
        let clients = self.inner.lock().await.clients.clone();
        let send_futures = clients
            .iter()
            .map(|client| client.send(msg.to_data()));

        // try to send to all clients, ignoring failures
        // disconnected clients will get swept up by `remove_stale_clients`
        let _ = future::join_all(send_futures).await;
    }
}