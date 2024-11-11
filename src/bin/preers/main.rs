mod app;
mod db;
mod http;
mod proxy;

use app::Network;
use clap::Parser;
use db::DataBase;
use libp2p::identity::{self, ed25519};
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};
use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::EnvFilter;

use preers::DEFAULT_HTTP_PORT;

type Responder<T> = oneshot::Sender<T>;


const DEFAULT_P2P_PORT: u16 = 0;
const MPSC_CHANNEL_SIZE: usize = 256;
const DEFAULT_DB_PATH: &str = "./preers.db";

#[derive(Parser)]
#[command(name = "preers")]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(
        short,
        long,
        default_value_t = DEFAULT_P2P_PORT,
        help = "port to listen on, default is any port currently unused (0)"
    )]
    port: u16,

    #[arg(long, help = "path to database, default is './preers.db'")]
    db: Option<PathBuf>,

    #[arg(long, default_value_t = DEFAULT_HTTP_PORT, help = "port for restful api")]
    http_port: u16,

    #[arg(long, help = "serve as a relay")]
    relay: bool,

    #[arg(long, help = "serve as a rendezvous point")]
    rendezvous: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let db_path = cli.db.unwrap_or(DEFAULT_DB_PATH.into());
    let db_created = db_path.exists();

    let mut db = DataBase::new(&db_path)?;
    println!("Opened database: {}", db.path().unwrap());
    let ed25519_keypair = if !db_created {
        db.init()?;
        println!("Database initialized");
        let kp = ed25519::Keypair::generate();
        db.set_setting("keypair", kp.to_bytes().as_ref())?;
        kp
    } else {
        let mut kp_bytes = db.get_setting("keypair")?;
        ed25519::Keypair::try_from_bytes(kp_bytes.as_mut_slice())?
    };

    let keypair = identity::Keypair::from(ed25519_keypair);

    println!("Peer ID: {}", keypair.public().to_peer_id());

    let rendezvous_list = db
        .get_rendezvous_list()?
        .into_iter()
        .map(|x| x.multiaddr)
        .collect();

    // TODO: handle intial rendezvous list and services together
    // Create libp2p application network eventloop
    let mut network = Network::new(keypair, cli.relay, cli.rendezvous, rendezvous_list)?;

    let used_services = db.get_used_services()?;
    let provided_services = db.get_provided_services()?;

    // Initialize network, start listening etc
    network.init(cli.port, used_services, provided_services)?;

    println!("Network initialized...");
    let (db_tx, db_rx) = mpsc::channel(MPSC_CHANNEL_SIZE);
    let (app_tx, app_rx) = mpsc::channel(MPSC_CHANNEL_SIZE);

    // Spawn sqlite database worker thread
    let _ = tokio::task::spawn_blocking(move || db.run(db_rx));
    println!("Database running...");

    // Spawn RESTful API http server
    tokio::spawn(http::serve_http(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        cli.http_port,
        db_tx.clone(),
        app_tx.clone(),
    ));
    println!("HTTP listening on {}", cli.http_port);

    network.run(app_rx, app_tx).await;

    Ok(())
}
