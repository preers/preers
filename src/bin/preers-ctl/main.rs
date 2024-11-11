use clap::{Parser, Subcommand, ValueEnum};

use libp2p::{Multiaddr, PeerId};
use preers::data::{ProvideService, UseService, Rendezvous, NetworkInfo};
use preers::DEFAULT_HTTP_PORT;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "preers-ctl")]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    port: Option<u16>,
    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Clone)]
enum Target {
    Rendezvous,
    Use,
    Provide
}

#[derive(Subcommand)]
enum Commands {
    Info,
    List {
        target: Target,
    },
    Add {
        target: Target,

        multiaddr: Option<String>,

        #[arg(long)]
        peer_id: Option<String>,
        
        #[arg(short = 'H', long)]
        host: Option<String>,

        #[arg(short, long)]
        port: Option<u16>,

        #[arg(short, long)]
        forwarder_port: Option<u16>,
    },
    Del {
        target: Target,
        id: i64,
    }
}

fn target_to_url(target: Target, port: u16) -> String {
    match target {
        Target::Rendezvous => format!("http://localhost:{port}/rendezvous"),
        Target::Use => format!("http://localhost:{port}/use_service"),
        Target::Provide => format!("http://localhost:{port}/provide_service")
    }
}

async fn list_cmd<T: DeserializeOwned + std::fmt::Debug>(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get(target_to_url(target, DEFAULT_HTTP_PORT))
        .await?
        .json::<T>()
        .await?;
    println!("{resp:#?}");
    Ok(())
}

async fn add_cmd<T: Serialize>(target: Target, object: T) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client.post(target_to_url(target, DEFAULT_HTTP_PORT))
        .json(&object)
        .send()
        .await?;
    println!("{resp:#?}");
    Ok(())
}

async fn info_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get(format!("http://localhost:{}/network_info", DEFAULT_HTTP_PORT))
        .await?
        .json::<NetworkInfo>()
        .await?;
    println!("{resp:#?}");
    Ok(())
}

async fn del_cmd<T: Serialize>(target: Target, object: T) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client.delete(target_to_url(target, DEFAULT_HTTP_PORT))
        .json(&object)
        .send()
        .await?;
    println!("{resp:#?}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_multiaddr = Multiaddr::empty();
    let default_peer_id = PeerId::random();

    let cli = Cli::parse();
    match cli.command {
        Commands::Info => info_cmd().await?,
        Commands::List { target } => {
            match target {
                Target::Rendezvous =>  list_cmd::<Vec<Rendezvous>>(target).await?,
                Target::Use =>  list_cmd::<Vec<UseService>>(target).await?,
                Target::Provide =>  list_cmd::<Vec<ProvideService>>(target).await?,
            }
        }
        Commands::Add { target, multiaddr, peer_id, host, port, forwarder_port } => {
            match target {
                Target::Rendezvous => {
                    if let Some(multiaddr) = multiaddr {
                        add_cmd(target, Rendezvous { id: 0, multiaddr: multiaddr.parse()? }).await?;
                    } else {
                        eprintln!("must provide multiaddr");
                    }
                }
                Target::Provide => {
                    if let (Some(host), Some(port)) = (host, port) {
                        add_cmd(target, ProvideService { id: 0, host, port }).await?;
                    } else {
                        eprintln!("must provide host and port");
                    }
                }
                Target::Use => {
                    if let (Some(peer_id), Some(host), Some(port), Some(forwarder_port)) = 
                        (peer_id, host, port, forwarder_port) {
                            add_cmd(target, UseService { id: 0, peer_id: peer_id.parse()?, host, port, forwarder_port }).await?;
                    } else {
                        eprintln!("must provide peer_id, host, port, and forwarder_port")
                    }
                }
            }
        }
        Commands::Del { target, id } => {
            match target {
                Target::Rendezvous => {
                    del_cmd(target, Rendezvous { id, multiaddr: default_multiaddr }).await?;
                }
                Target::Use => {
                    del_cmd(target, UseService { id, peer_id: default_peer_id, host: "".to_string(), port: 0, forwarder_port: 0}).await?;
                }
                Target::Provide => {
                    del_cmd(target, ProvideService { id, host: "".to_string(), port: 0 }).await?;
                }
            }
        }
    }
    Ok(())
}
