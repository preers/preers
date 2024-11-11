use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::net::{IpAddr, SocketAddr};
use tokio::sync::{
    mpsc::{error::SendError, Sender},
    oneshot::{self, error::RecvError},
};

use preers::data::{NetworkInfo, ProvideService, Rendezvous, UseService};
use crate::{
    app,
    db::{self, AddInner, DelInner},
};

#[derive(Clone)]
struct AppState {
    db_tx: Sender<db::Command>,
    app_tx: Sender<app::Command>,
}

#[derive(Debug)]
enum Error {
    SendError,
    RecvError,
    DBError,
}

type Result<T> = std::result::Result<T, Error>;

pub async fn serve_http(
    host: IpAddr,
    port: u16,
    db_tx: Sender<db::Command>,
    app_tx: Sender<app::Command>,
) {
    let app_state = AppState { db_tx, app_tx };
    let app = Router::new()
        .route("/network_info", get(get_info))
        .route(
            "/rendezvous",
            get(get_rendezvous)
                .post(post_rendezvous)
                .delete(delete_rendezvous),
        )
        .route(
            "/provide_service",
            get(get_provide_service)
                .post(post_provide_service)
                .delete(delete_provide_service),
        )
        .route(
            "/use_service",
            get(get_use_service)
                .post(post_use_service)
                .delete(delete_use_service),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(SocketAddr::new(host, port))
        .await
        .expect("HTTP server should be able to listen.");
    axum::serve(listener, app)
        .await
        .expect("HTTP server should serve.");
}

async fn get_info(State(AppState { app_tx, .. }): State<AppState>) -> Result<Json<NetworkInfo>> {
    tracing::debug!("getting network info");
    let (resp_tx, resp_rx) = oneshot::channel();
    app_tx.send(app::Command::GetNetworkInfo(resp_tx)).await?;
    Ok(Json(resp_rx.await?))
}

async fn get_rendezvous(
    State(AppState { db_tx, .. }): State<AppState>,
) -> Result<Json<Vec<Rendezvous>>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx.send(db::Command::GetRendezvous(resp_tx)).await?;
    Ok(Json(resp_rx.await??))
}

async fn post_rendezvous(
    State(AppState { db_tx, app_tx }): State<AppState>,
    Json(mut rendezvous): Json<Rendezvous>,
) -> Result<Json<Rendezvous>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::Add {
            inner: AddInner::Rendezvous(rendezvous.multiaddr.clone()),
            resp: resp_tx,
        })
        .await?;

    app_tx
        .send(app::Command::AddRendezvous(rendezvous.multiaddr.clone()))
        .await?;

    let rendezvous_id = resp_rx.await?;
    rendezvous.id = rendezvous_id?;
    Ok(Json(rendezvous))
}

// TODO: delete rendezvous realtime
async fn delete_rendezvous(
    State(AppState { db_tx, .. }): State<AppState>,
    Json(rendezvous): Json<Rendezvous>,
) -> Result<()> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::Del {
            inner: DelInner::Rendezvous(rendezvous.id),
            resp: resp_tx,
        })
        .await?;
    Ok(resp_rx.await??)
}

async fn get_provide_service(
    State(AppState { db_tx, .. }): State<AppState>,
) -> Result<Json<Vec<ProvideService>>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::GetProvidedServices(resp_tx))
        .await?;
    Ok(Json(resp_rx.await??))
}

async fn post_provide_service(
    State(AppState { db_tx, app_tx }): State<AppState>,
    Json(mut provide_service): Json<ProvideService>,
) -> Result<Json<ProvideService>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::Add {
            inner: AddInner::ProvideService(provide_service.clone()),
            resp: resp_tx,
        })
        .await?;

    app_tx
        .send(app::Command::ProvideService(provide_service.clone()))
        .await?;

    let id = resp_rx.await?;
    provide_service.id = id?;
    Ok(Json(provide_service))
}

async fn delete_provide_service(
    State(AppState { db_tx, .. }): State<AppState>,
    Json(provide_service): Json<ProvideService>,
) -> Result<()> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::Del {
            inner: DelInner::ProvideService(provide_service.id),
            resp: resp_tx,
        })
        .await?;
    Ok(resp_rx.await??)
}

async fn get_use_service(
    State(AppState { db_tx, .. }): State<AppState>,
) -> Result<Json<Vec<UseService>>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx.send(db::Command::GetUsedServices(resp_tx)).await?;
    Ok(Json(resp_rx.await??))
}

async fn post_use_service(
    State(AppState { db_tx, app_tx }): State<AppState>,
    Json(mut use_service): Json<UseService>,
) -> Result<Json<UseService>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::Add {
            inner: AddInner::UseService(use_service.clone()),
            resp: resp_tx,
        })
        .await?;

    app_tx
        .send(app::Command::UseService(use_service.clone()))
        .await?;

    let id = resp_rx.await?;
    use_service.id = id?;
    Ok(Json(use_service))
}

async fn delete_use_service(
    State(AppState { db_tx, .. }): State<AppState>,
    Json(use_service): Json<UseService>,
) -> Result<()> {
    let (resp_tx, resp_rx) = oneshot::channel();
    db_tx
        .send(db::Command::Del {
            inner: DelInner::UseService(use_service.id),
            resp: resp_tx,
        })
        .await?;
    Ok(resp_rx.await??)
}

// TODO: Better error handling here.
impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Self::SendError
    }
}

impl From<RecvError> for Error {
    fn from(_: RecvError) -> Self {
        Self::RecvError
    }
}

impl From<db::Error> for Error {
    fn from(_: db::Error) -> Self {
        Self::DBError
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
