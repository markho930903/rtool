use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant, sleep};

use super::TRANSFER_DISCOVERY_PORT;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryPacket {
    pub device_id: String,
    pub display_name: String,
    pub listen_port: u16,
    pub app_version: String,
    pub pairing_required: bool,
    pub capabilities: Vec<String>,
    pub ts: i64,
}

#[derive(Debug, Clone)]
pub struct DiscoveryPeer {
    pub device_id: String,
    pub display_name: String,
    pub address: String,
    pub listen_port: u16,
    pub last_seen_at: i64,
    pub pairing_required: bool,
}

pub type PeerMap = Arc<RwLock<HashMap<String, DiscoveryPeer>>>;

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
}

pub async fn run_broadcast_loop(stop: Arc<AtomicBool>, packet: DiscoveryPacket) {
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(
                event = "transfer_discovery_broadcast_bind_failed",
                error = error.to_string()
            );
            return;
        }
    };

    if let Err(error) = socket.set_broadcast(true) {
        tracing::error!(
            event = "transfer_discovery_broadcast_enable_failed",
            error = error.to_string()
        );
        return;
    }

    let target = SocketAddr::from(([255, 255, 255, 255], TRANSFER_DISCOVERY_PORT));
    while !stop.load(Ordering::Relaxed) {
        let mut payload = packet.clone();
        payload.ts = now_millis();

        match serde_json::to_vec(&payload) {
            Ok(bytes) => {
                if let Err(error) = socket.send_to(bytes.as_slice(), target).await {
                    tracing::warn!(
                        event = "transfer_discovery_broadcast_send_failed",
                        error = error.to_string()
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    event = "transfer_discovery_broadcast_serialize_failed",
                    error = error.to_string()
                );
            }
        }

        sleep(Duration::from_secs(3)).await;
    }
}

pub async fn run_listen_loop(stop: Arc<AtomicBool>, peers: PeerMap, local_device_id: String) {
    let socket = match UdpSocket::bind(("0.0.0.0", TRANSFER_DISCOVERY_PORT)).await {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(
                event = "transfer_discovery_listen_bind_failed",
                error = error.to_string()
            );
            return;
        }
    };

    let mut buffer = vec![0u8; 4096];
    let mut prune_deadline = Instant::now() + Duration::from_secs(10);

    while !stop.load(Ordering::Relaxed) {
        tokio::select! {
            recv = socket.recv_from(buffer.as_mut_slice()) => {
                let Ok((size, addr)) = recv else {
                    continue;
                };
                if size == 0 {
                    continue;
                }

                let payload = &buffer[..size];
                let Ok(packet) = serde_json::from_slice::<DiscoveryPacket>(payload) else {
                    continue;
                };
                if packet.device_id == local_device_id {
                    continue;
                }

                let peer = DiscoveryPeer {
                    device_id: packet.device_id,
                    display_name: packet.display_name,
                    address: addr.ip().to_string(),
                    listen_port: packet.listen_port,
                    last_seen_at: now_millis(),
                    pairing_required: packet.pairing_required,
                };

                peers.write().await.insert(peer.device_id.clone(), peer);
            }
            _ = sleep(Duration::from_millis(300)) => {}
        }

        if Instant::now() >= prune_deadline {
            let now = now_millis();
            peers
                .write()
                .await
                .retain(|_, peer| now - peer.last_seen_at <= 10_000);
            prune_deadline = Instant::now() + Duration::from_secs(2);
        }
    }
}
