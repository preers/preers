use libp2p::{multiaddr::Multiaddr, PeerId};
use rusqlite::{Connection, Result};
use std::path::Path;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing;

pub use rusqlite::Error;

use crate::{
    data::{ProvideService, Rendezvous, UseService},
    Responder,
};

pub(crate) enum Command {
    Add {
        inner: AddInner,
        resp: Responder<Result<i64>>,
    },
    Del {
        inner: DelInner,
        resp: Responder<Result<()>>,
    },
    GetRendezvous(Responder<Result<Vec<Rendezvous>>>),
    GetUsedServices(Responder<Result<Vec<UseService>>>),
    GetProvidedServices(Responder<Result<Vec<ProvideService>>>),
}

pub(crate) enum AddInner {
    Rendezvous(Multiaddr),
    ProvideService(ProvideService),
    UseService(UseService),
}

pub(crate) enum DelInner {
    Rendezvous(i64),
    ProvideService(i64),
    UseService(i64),
}

pub(crate) struct DataBase {
    conn: Connection,
}

impl DataBase {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn init(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE settings (
                key     TEXT PRIMARY KEY,
                value   BLOB
            );
            CREATE TABLE rendezvous ( multiaddr PRIMARY KEY );
            CREATE TABLE provided_services (
                host    TEXT DEFAULT('localhost'), 
                port    INTEGER NOT NULL
            );
            CREATE TABLE used_services (
                peer_id             TEXT NOT NULL,
                host                TEXT DEFAULT('localhost'),
                port                INTEGER NOT NULL,
                forwarder_port      INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn set_setting(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) 
                VALUES(?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (key, value),
        )?;
        Ok(())
    }

    pub fn get_setting(&mut self, key: &str) -> Result<Vec<u8>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        stmt.query_row([key], |row| row.get(0))
    }

    pub fn get_rendezvous_list(&mut self) -> Result<Vec<Rendezvous>> {
        let mut stmt = self
            .conn
            .prepare("SELECT rowid, multiaddr FROM rendezvous")?;
        let rendezvous_list = stmt.query_map([], |row| {
            Ok(Rendezvous {
                id: row.get(0)?,
                multiaddr: Multiaddr::from_str(&row.get::<usize, String>(1)?)
                    .expect("should parse"),
            })
        })?;
        let mut v = Vec::new();
        for get_result in rendezvous_list {
            match get_result {
                Ok(rendezvous) => v.push(rendezvous),
                Err(error) => tracing::error!(?error, "getting rendezvous row error"),
            }
        }
        Ok(v)
    }

    pub fn get_used_services(&mut self) -> Result<Vec<UseService>> {
        let mut stmt = self
            .conn
            .prepare("SELECT rowid, peer_id, host, port, forwarder_port FROM used_services")?;
        let used_services = stmt.query_map([], |row| {
            Ok(UseService {
                id: row.get(0)?,
                peer_id: PeerId::from_str(&row.get::<usize, String>(1)?).expect("should parse"),
                host: row.get(2)?,
                port: row.get(3)?,
                forwarder_port: row.get(4)?,
            })
        })?;
        let mut v = Vec::new();
        for get_result in used_services {
            match get_result {
                Ok(use_service) => v.push(use_service),
                Err(error) => tracing::error!(?error, "getting used_services row error"),
            }
        }
        Ok(v)
    }

    // TODO: deal with duplicate service
    pub fn add_used_service(&mut self, service: &UseService) -> Result<i64> {
        let _ = self.conn.execute(
            "INSERT INTO
            used_services (peer_id, host, port, forwarder_port)
            VALUES (?1, ?2, ?3, ?4)",
            (
                service.peer_id.to_base58(),
                &service.host,
                service.port,
                service.forwarder_port,
            ),
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_provided_services(&mut self) -> Result<Vec<ProvideService>> {
        let mut stmt = self
            .conn
            .prepare("SELECT rowid, host, port FROM provided_services")?;
        let provided_services = stmt.query_map([], |row| {
            Ok(ProvideService {
                id: row.get(0)?,
                host: row.get(1)?,
                port: row.get(2)?,
            })
        })?;
        let mut v = Vec::new();
        for get_result in provided_services {
            match get_result {
                Ok(provide_service) => v.push(provide_service),
                Err(error) => tracing::error!(?error, "getting provided_services row error"),
            }
        }
        Ok(v)
    }

    // TODO: deal with duplicate servcie
    pub fn add_provided_service(&mut self, service: &ProvideService) -> Result<i64> {
        let _ = self.conn.execute(
            "INSERT INTO
            provided_services ( host, port)
            VALUES (?1, ?2)",
            (&service.host, service.port),
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn handle_add(&mut self, cmd: AddInner) -> Result<i64> {
        match cmd {
            AddInner::Rendezvous(multiaddr) => {
                self.conn.execute(
                    "INSERT INTO rendezvous (mutiaddr) VALUES (?1)",
                    [multiaddr.to_string()],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
            AddInner::UseService(use_servcie) => self.add_used_service(&use_servcie),
            AddInner::ProvideService(provide_service) => {
                self.add_provided_service(&provide_service)
            }
        }
    }

    pub fn delete_with_id(&mut self, table: &str, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM ?1 WHERE rowid = ?2", (table, id))?;
        Ok(())
    }

    pub fn handle_del(&mut self, cmd: DelInner) -> Result<()> {
        match cmd {
            DelInner::Rendezvous(id) => self.delete_with_id("rendezvous", id),
            DelInner::UseService(id) => self.delete_with_id("used_services", id),
            DelInner::ProvideService(id) => self.delete_with_id("provided_services", id),
        }
    }

    pub fn run(mut self, mut rx: mpsc::Receiver<Command>) {
        loop {
            if let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    Command::Add { inner, resp } => {
                        resp.send(self.handle_add(inner));
                    }
                    Command::Del { inner, resp } => {
                        resp.send(self.handle_del(inner));
                    }
                    Command::GetRendezvous(resp) => {
                        resp.send(self.get_rendezvous_list());
                    }
                    Command::GetUsedServices(resp) => {
                        resp.send(self.get_used_services());
                    }
                    Command::GetProvidedServices(resp) => {
                        resp.send(self.get_provided_services());
                    }
                }
            } else {
                break;
            }
        }
    }

    pub fn path(&self) -> Option<&str> {
        self.conn.path()
    }
}
