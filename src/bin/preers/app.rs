use libp2p::{
    core::{multiaddr::Protocol, ConnectedPoint},
    dcutr,
    futures::StreamExt,
    identify,
    identity::Keypair,
    noise, ping, relay,
    rendezvous::{self, Cookie, Namespace},
    swarm::{
        behaviour::toggle::Toggle, dial_opts::DialOpts, ConnectionId, NetworkBehaviour, Swarm,
        SwarmEvent,
    },
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};

use libp2p_stream as stream;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::time::Duration;

use crate::{proxy, MPSC_CHANNEL_SIZE, Responder};
use tokio::sync::mpsc;

use preers::data::{NetworkInfo, PeerInfo, ProvideService, UseService};

// default rendezvous registration ttl is 2 hours
const DEFAULT_RDV_REGISTRATION_TTL: Duration = Duration::from_secs(2 * 60 * 60);

// default time interval for rendezvous registration renewal and discovery
const DEFAULT_RDV_REFRESH: Duration = Duration::from_secs(60 * 60);

#[derive(NetworkBehaviour)]
struct Behaviour {
    identify: identify::Behaviour,
    dcutr: dcutr::Behaviour,
    relay_client: relay::client::Behaviour,
    rendezvous_client: rendezvous::client::Behaviour,
    ping: ping::Behaviour,
    stream: stream::Behaviour,
    relay: Toggle<relay::Behaviour>,
    rendezvous: Toggle<rendezvous::server::Behaviour>,
}

pub(crate) enum Command {
    AddRendezvous(Multiaddr),
    AddRelay(PeerId),
    TalkToRendezvous(PeerId),
    GetNetworkInfo(Responder<NetworkInfo>),
    UseService(UseService),
    ProvideService(ProvideService),
}

pub(crate) struct Network {
    swarm: Swarm<Behaviour>,
    rendezvous_list: Vec<Multiaddr>,
    is_relay: bool,
    is_rendezvous: bool,
    pending_relay_connections: HashSet<ConnectionId>,
    pending_rendezvous_connections: HashSet<ConnectionId>,
    // rendezvous request cookies
    rdv_cookies: HashMap<(PeerId, Option<Namespace>), Cookie>,
    // peers we ever connected to
    peers: HashSet<PeerId>,
    // channel to handle provide service requests
    provide_service_tx: mpsc::Sender<ProvideService>,
}

impl Network {
    pub fn new(
        keypair: Keypair,
        is_relay: bool,
        is_rendezvous: bool,
        rendezvous_list: Vec<Multiaddr>,
    ) -> Result<Self, Box<dyn Error>> {
        let peer_id = keypair.public().to_peer_id();
        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_quic()
            .with_dns()?
            .with_relay_client(noise::Config::new, yamux::Config::default)?
            .with_behaviour(|keypair, relay_behaviour| Behaviour {
                identify: identify::Behaviour::new(identify::Config::new(
                    "/preers/id/1.0.0".to_string(),
                    keypair.public(),
                )),
                dcutr: dcutr::Behaviour::new(peer_id),
                relay_client: relay_behaviour,
                rendezvous_client: rendezvous::client::Behaviour::new(keypair.clone()),
                ping: ping::Behaviour::new(ping::Config::new()),
                stream: stream::Behaviour::new(),
                relay: (if is_relay {
                    Some(relay::Behaviour::new(peer_id, relay::Config::default()))
                } else {
                    None
                })
                .into(),
                rendezvous: (if is_rendezvous {
                    Some(rendezvous::server::Behaviour::new(
                        rendezvous::server::Config::default(),
                    ))
                } else {
                    None
                })
                .into(),
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(2 * 60 * 60)))
            .build();

        let (provide_service_tx, provide_service_rx) = mpsc::channel(MPSC_CHANNEL_SIZE);

        // spawn provide services, handle incoming requests
        tokio::spawn(proxy::provide_services(
            provide_service_rx,
            swarm.behaviour().stream.new_control(),
        ));

        Ok(Self {
            swarm,
            rendezvous_list,
            is_relay,
            is_rendezvous,
            pending_relay_connections: Default::default(),
            pending_rendezvous_connections: Default::default(),
            rdv_cookies: Default::default(),
            peers: Default::default(),
            provide_service_tx,
        })
    }

    pub fn init(
        &mut self,
        port: u16,
        used_services: Vec<UseService>,
        provided_services: Vec<ProvideService>,
    ) -> Result<(), Box<dyn Error>> {
        let addrs = vec![
            format!("/ip4/0.0.0.0/tcp/{port}"),
            format!("/ip6/::/tcp/{port}"),
            format!("/ip4/0.0.0.0/udp/{port}/quic-v1"),
            format!("/ip6/::/udp/{port}/quic-v1"),
        ];
        for addr in addrs {
            tracing::info!(%addr, "listen on");
            let multiaddr = addr.parse().unwrap();
            self.swarm.listen_on(multiaddr)?;
        }

        // add known rendezvous
        // TODO: work around the clone here
        for rendezvous in self.rendezvous_list.clone().into_iter() {
            self.add_rendezvous(&rendezvous);
        }

        // add used services
        for use_service in used_services {
            tokio::spawn(proxy::use_service(
                use_service,
                self.swarm.behaviour().stream.new_control(),
            ));
        }

        // add known provided services
        for provide_service in provided_services {
            // TODO handle send error
            self.provide_service_tx.try_send(provide_service);
        }
        Ok(())
    }

    pub async fn run(mut self, mut app_rx: mpsc::Receiver<Command>, app_tx: mpsc::Sender<Command>) {
        loop {
            tokio::select! {
                Some(command) = app_rx.recv() => {
                    self.handle_command(command);
                }
                event = self.swarm.select_next_some() => {
                    self.handle_event(event, &app_tx);
                }
                else => {
                    break;
                }
            }
        }
    }

    fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent>, app_tx: &mpsc::Sender<Command>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                // NOTE: explicitly add every listening address in order to expose our LAN address
                // to peers for direct LAN connectivity
                self.swarm.add_external_address(address.clone());
                tracing::info!(%address, "new listen address");
            }

            SwarmEvent::ConnectionClosed {
                peer_id,
                cause: Some(error),
                ..
            } => {
                tracing::info!(%peer_id, ?error, "conneciton closed with error");
            }

            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                ..
            } => {
                self.peers.insert(peer_id);
                if let Some(_) = self.pending_rendezvous_connections.take(&connection_id) {
                    tokio::spawn(talk_to_rendezvous(app_tx.clone(), peer_id));
                    tracing::info!(rendezvous_point = %peer_id, "connected to rendezvous");
                }

                if let Some(_) = self.pending_relay_connections.take(&connection_id) {
                    tracing::info!(relay = %peer_id, "connected to relay");
                    // TODO: deal with multiple relays
                    // TODO: register this address with rendezvous
                    if let ConnectedPoint::Dialer { address, .. } = endpoint {
                        if let Err(error) = self
                            .swarm
                            .listen_on(address.clone().with(Protocol::P2pCircuit))
                        {
                            tracing::error!(relay = %peer_id, %address, ?error, "listen on circuit relay address error");
                        } else {
                            tracing::info!(relay = %peer_id, %address, "listening on circuit relay address");
                        }
                    }
                }
            }

            SwarmEvent::OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => {
                if let Some(_) = self.pending_relay_connections.take(&connection_id) {
                    tracing::error!(relay = ?peer_id, ?error, "connetion to relay server error");
                }
                if let Some(_) = self.pending_rendezvous_connections.take(&connection_id) {
                    tracing::error!(rendezvous_point = ?peer_id, ?error, "connection to rendezvous point error");
                }
            }

            // once `/identify` did its job, we know our external address and can register
            SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
                rendezvous::client::Event::Registered {
                    namespace,
                    ttl,
                    rendezvous_node: rendezvous_point,
                },
            )) => {
                tracing::info!(%rendezvous_point, %namespace, %ttl, "successfully registered at rendezvout point");
            }

            SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
                rendezvous::client::Event::RegisterFailed {
                    rendezvous_node: rendezvous_point,
                    namespace,
                    error,
                },
            )) => {
                tracing::error!(%rendezvous_point, %namespace, ?error, "failed to register at rendezvout point");
            }

            SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
                rendezvous::client::Event::Discovered {
                    registrations,
                    cookie: new_cookie,
                    rendezvous_node,
                },
            )) => {
                let maybe_namespace = new_cookie.namespace();
                if let Some(cookie) = self
                    .rdv_cookies
                    .get_mut(&(rendezvous_node, maybe_namespace.cloned()))
                {
                    *cookie = new_cookie.clone();
                } else {
                    self.rdv_cookies.insert(
                        (rendezvous_node, maybe_namespace.cloned()),
                        new_cookie.clone(),
                    );
                }
                // Only dial relay immediately. Peers' addresses are maintained by rendezvous
                // client behaviour itself, when dialing a peer by id, the swarm will get those
                // addresses by extend_addresses_through_behaviour
                if maybe_namespace.is_some_and(|ns| ns == "relay") {
                    for registration in registrations {
                        app_tx.send(Command::AddRelay(registration.record.peer_id()));
                    }
                }
            }

            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted {
                    relay_peer_id: relay,
                    renewal,
                    ..
                },
            )) => {
                tracing::info!(%renewal, %relay, "relay accepted our reservation");
            }

            SwarmEvent::Behaviour(BehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            })) => {
                tracing::info!(peer_id = %peer, ?rtt, "ping to peer success")
            }

            other => {
                tracing::debug!("Unhandled {:?}", other);
            }
        }
    }

    fn add_rendezvous(&mut self, rendezvous_point: &Multiaddr) {
        let dial_opts = DialOpts::unknown_peer_id()
            .address(rendezvous_point.clone())
            .build();
        let connection_id = dial_opts.connection_id();
        if let Err(error) = self.swarm.dial(dial_opts) {
            tracing::error!(?error, %rendezvous_point, "dial rendezvous point error");
        } else {
            self.pending_rendezvous_connections.insert(connection_id);
            tracing::info!(%rendezvous_point, "dialing rendezvous point");
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::AddRendezvous(rendezvous_point) => {
                self.add_rendezvous(&rendezvous_point);
                self.rendezvous_list.push(rendezvous_point);
            }
            Command::AddRelay(relay) => {
                if self.is_relay {
                    return;
                }
                let dial_opts = DialOpts::peer_id(relay).build();
                let connection_id = dial_opts.connection_id();
                if let Err(error) = self.swarm.dial(dial_opts) {
                    tracing::error!(?error, %relay, "dial relay server error");
                } else {
                    self.pending_relay_connections.insert(connection_id);
                    tracing::info!(%relay, "dialing relay server");
                }
            }
            Command::TalkToRendezvous(rendezvous_point) => {
                // Register as preers
                if let Err(error) = self.swarm.behaviour_mut().rendezvous_client.register(
                    rendezvous::Namespace::from_static("preers"),
                    rendezvous_point,
                    Some(DEFAULT_RDV_REGISTRATION_TTL.as_secs()),
                ) {
                    tracing::error!(%rendezvous_point, ?error, "failed to register as preers");
                } else {
                    tracing::info!(%rendezvous_point, "registering as preers");
                }

                // Register as relay
                if self.is_relay {
                    if let Err(error) = self.swarm.behaviour_mut().rendezvous_client.register(
                        rendezvous::Namespace::from_static("relay"),
                        rendezvous_point,
                        Some(DEFAULT_RDV_REGISTRATION_TTL.as_secs()),
                    ) {
                        tracing::error!(%rendezvous_point, ?error, "failed to register as relay");
                    } else {
                        tracing::info!(%rendezvous_point, "registering as relay");
                    }
                }

                // Discover relay nodes
                // TODO: use another way to determine if we have to discover relays
                if !self.is_relay && !self.is_rendezvous {
                    self.swarm.behaviour_mut().rendezvous_client.discover(
                        Some(rendezvous::Namespace::from_static("relay")),
                        self.rdv_cookies
                            .get(&(rendezvous_point, Some(Namespace::from_static("relay"))))
                            .cloned(),
                        None,
                        rendezvous_point,
                    );
                }

                // Discover preers
                self.swarm.behaviour_mut().rendezvous_client.discover(
                    Some(rendezvous::Namespace::new("preers".to_string()).unwrap()),
                    self.rdv_cookies
                        .get(&(rendezvous_point, Some(Namespace::from_static("preers"))))
                        .cloned(),
                    None,
                    rendezvous_point,
                );
            }
            Command::GetNetworkInfo(resp) => {
                resp.send(NetworkInfo {
                    peer_id: *self.swarm.local_peer_id(),
                    peers: self
                        .peers
                        .iter()
                        .map(|peer_id| PeerInfo {
                            peer_id: *peer_id,
                            connected: self.swarm.is_connected(&peer_id),
                        })
                        .collect(),
                });
            }
            Command::UseService(use_service) => {
                tokio::spawn(proxy::use_service(
                    use_service,
                    self.swarm.behaviour().stream.new_control(),
                ));
            }
            Command::ProvideService(provide_service) => {
                // TODO: handle send error
                self.provide_service_tx.try_send(provide_service);
            }
        }
    }
}

// TODO: need to be able to cancel this (when a rendezvous address is deleted)
async fn talk_to_rendezvous(app_tx: mpsc::Sender<Command>, peer_id: PeerId) {
    loop {
        if app_tx
            .send(Command::TalkToRendezvous(peer_id))
            .await
            .is_err()
        {
            break;
        }
        tokio::time::sleep(DEFAULT_RDV_REFRESH).await;
    }
}
