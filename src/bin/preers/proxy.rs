use preers::data::{ProvideService, UseService};
use asynchronous_codec::Framed;
use futures::{AsyncReadExt, SinkExt, StreamExt};
use libp2p::{Stream, StreamProtocol};
use libp2p_stream as stream;
use pin_project::pin_project;
use std::collections::HashSet;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::result::Result;
use std::sync::{Arc, Mutex};
use tokio::io::copy_bidirectional;
use tokio::io::ReadBuf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

mod proto {
    include!("generated/mod.rs");
    pub(crate) use self::proxy::pb::{UseServiceReq, UseServiceResp};
}

const PROXY_PROTOCOL: StreamProtocol = StreamProtocol::new("/preers-proxy");
const MAX_MESSAGE_SIZE: usize = 1024;

pub async fn use_service(use_service: UseService, mut control: stream::Control) {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, use_service.forwarder_port)).await else {
        tracing::error!(?use_service, "listen local error");
        return;
    };
    tracing::info!(?use_service, "listening on local host");
    loop {
        match listener.accept().await {
            Ok((local_stream, from_addr)) => {
                tracing::info!(?use_service, %from_addr, "accepted incoming request");
                if let Ok(remote_stream) = control
                    .open_stream(use_service.peer_id, PROXY_PROTOCOL)
                    .await
                {
                    let _ = tokio::spawn(handle_outbound(
                        local_stream,
                        remote_stream,
                        use_service.host.clone(),
                        use_service.port,
                    ));
                } else {
                    tracing::error!(peer_id = %use_service.peer_id, "open stream error");
                    return;
                }
            }
            Err(error) => {
                tracing::error!(?error, "accept stream error");
                // TODO just return or handle error and continue?
                return;
            }
        }
    }
}

async fn handle_outbound(
    mut local_stream: TcpStream,
    remote_stream: Stream,
    host: String,
    port: u16,
) {
    let mut framed_stream = Framed::new(
        remote_stream,
        quick_protobuf_codec::Codec::new(MAX_MESSAGE_SIZE),
    );
    let msg = proto::UseServiceReq {
        host,
        port: port as u32,
    };
    tracing::debug!(?msg, "sending request");
    if let Err(error) = framed_stream.send(msg).await {
        tracing::error!(?error, "proxy send initial msg faild");
        return;
    }
    tracing::debug!("sent request");
    let Some(Ok(proto::UseServiceResp { allowed })) = framed_stream.next().await else {
        tracing::error!("receive use service response error");
        return;
    };
    if !allowed {
        tracing::error!("use service not allowed by remote");
        return;
    }
    let remote_stream = framed_stream.into_inner();

    // Convert remote_stream to imple tokio AsyncRead and AsyncWrite
    // TODO: redeem this atrocity...
    let (remote_read, remote_write) = remote_stream.split();
    let remote_read = remote_read.compat();
    let remote_write = remote_write.compat_write();
    let mut remote_stream = TokioReadWrite {
        reader: remote_read,
        writer: remote_write,
    };

    // Copy between remote and local
    let Ok((local_to_remote, remote_to_local)) =
        copy_bidirectional(&mut local_stream, &mut remote_stream).await
    else {
        tracing::error!("proxy error");
        return;
    };
    tracing::info!(%local_to_remote, %remote_to_local, "proxing done successfully");
}

pub async fn provide_services(
    mut rx: mpsc::Receiver<ProvideService>,
    mut control: stream::Control,
) {
    let mut incoming = control
        .accept(PROXY_PROTOCOL)
        .expect("should get incoming streams");
    let provided_services = Arc::new(Mutex::new(HashSet::new()));
    loop {
        tokio::select! {
            Some((peer_id, stream)) = incoming.next() => {
                tracing::info!(%peer_id, "incoming use service request from peer");
                tokio::spawn(handle_inbound(provided_services.clone(), stream));
            }
            Some(ProvideService { host, port, .. }) = rx.recv() => {
                provided_services.lock().unwrap().insert((host, port));
            }
            else => {
                break;
            }
        }
    }
}

async fn handle_inbound(
    provided_services: Arc<Mutex<HashSet<(String, u16)>>>,
    remote_stream: Stream,
) {
    let mut framed_stream = Framed::new(
        remote_stream,
        quick_protobuf_codec::Codec::new(MAX_MESSAGE_SIZE),
    );
    let Some(Ok(proto::UseServiceReq { host, port })) = framed_stream.next().await else {
        tracing::error!("receive use service request error");
        return;
    };
    tracing::debug!(%host, %port, "received use service request from peer");
    if provided_services
        .lock()
        .unwrap()
        .get(&(host.to_string(), port as u16))
        .is_none()
    {
        tracing::warn!(%host, %port, "incoming service request not allowed");
        framed_stream
            .send(proto::UseServiceResp { allowed: false })
            .await;
        framed_stream.close();
        return;
    }
    let Ok(socketaddr) = format!("{host}:{port}").parse::<SocketAddr>() else {
        tracing::warn!(%host, %port, "incoming request not valid");
        framed_stream
            .send(proto::UseServiceResp { allowed: false })
            .await;
        framed_stream.close();
        return;
    };
    if let Err(error) = framed_stream
        .send(proto::UseServiceResp { allowed: true })
        .await
    {
        tracing::warn!(?error, "send use service response to remote error");
        return;
    }
    let remote_stream = framed_stream.into_inner();
    let Ok(mut local_stream) = TcpStream::connect(socketaddr).await else {
        tracing::error!("connect to provided service error");
        return;
    };

    // Convert remote_stream to imple tokio AsyncRead and AsyncWrite
    let (remote_read, remote_write) = remote_stream.split();
    let remote_read = remote_read.compat();
    let remote_write = remote_write.compat_write();
    let mut remote_stream = TokioReadWrite {
        reader: remote_read,
        writer: remote_write,
    };

    // Copy between remote and local
    let Ok((local_to_remote, remote_to_local)) =
        copy_bidirectional(&mut local_stream, &mut remote_stream).await
    else {
        tracing::error!("proxy error");
        return;
    };
    tracing::info!(%local_to_remote, %remote_to_local, "proxing done successfully");
}

#[pin_project]
struct TokioReadWrite<R, W> {
    #[pin]
    reader: R,
    #[pin]
    writer: W,
}

impl<R: tokio::io::AsyncRead, W: tokio::io::AsyncWrite> tokio::io::AsyncRead
    for TokioReadWrite<R, W>
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let mut this = self.project();
        this.reader.as_mut().poll_read(cx, buf)
    }
}

impl<R: tokio::io::AsyncRead, W: tokio::io::AsyncWrite> tokio::io::AsyncWrite
    for TokioReadWrite<R, W>
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, io::Error>> {
        let mut this = self.project();
        this.writer.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        let this = self.project();
        this.writer.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        let mut this = self.project();
        this.writer.as_mut().poll_shutdown(cx)
    }
}
