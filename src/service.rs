use std::net::IpAddr;

pub struct ProvideService {
    host: IpAddr,
    port: u16,
}

pub struct UseService {
    peer_name: String,
    host: IpAddr,
    port: u16,
    forwarder_port: u16,
}
