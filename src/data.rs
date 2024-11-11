use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Rendezvous {
    pub id: i64,
    pub multiaddr: Multiaddr,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub connected: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkInfo {
    pub peer_id: PeerId,
    pub peers: Vec<PeerInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UseService {
    pub id: i64,
    pub peer_id: PeerId,
    pub host: String,
    pub port: u16,
    pub forwarder_port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProvideService {
    pub id: i64,
    pub host: String,
    pub port: u16,
}
