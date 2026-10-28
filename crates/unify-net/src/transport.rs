use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rustls::{ClientConfig, ServerConfig};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};
use unify_proto::{decode_payload, encode_frame, frame::parse_frame_len, InputEvent, Message, ProtoError, FRAME_HEADER_LEN};
use unify_security::tls::{peer_fingerprint, server_name, PAIRING_EXPORTER_LABEL};
use unify_security::Fingerprint;

use crate::NetError;

pub const HIGH_PRIORITY_CAPACITY: usize = 8_192;
pub const LOW_PRIORITY_CAPACITY: usize = 64;
const MOVE_COALESCE_LIMIT: i32 = 1 << 19;

pub type SessionStream = TlsStream<TcpStream>;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub fingerprint: Fingerprint,
    pub certificate: Vec<u8>,
    pub remote_addr: SocketAddr,
    pub channel_binding: [u8; 32],
}

fn peer_info(state: &rustls::CommonState, exporter: [u8; 32], remote_addr: SocketAddr) -> Result<PeerInfo, NetError> {
    let (fingerprint, certificate) = peer_fingerprint(state).ok_or(NetError::NoPeerCertificate)?;
    Ok(PeerInfo { fingerprint, certificate, remote_addr, channel_binding: exporter })
}

pub async fn connect_tls(address: &str, config: Arc<ClientConfig>, connect_timeout: Duration, handshake_timeout: Duration) -> Result<(SessionStream, PeerInfo), NetError> {
    let candidates: Vec<SocketAddr> = tokio::time::timeout(connect_timeout, tokio::net::lookup_host(address))
        .await
        .map_err(|_| NetError::Timeout)?
        .map_err(|_| NetError::Resolve)?
        .collect();
    if candidates.is_empty() {
        return Err(NetError::Resolve);
    }
    let mut last_error = NetError::Resolve;
    for candidate in candidates {
        match tokio::time::timeout(connect_timeout, TcpStream::connect(candidate)).await {
            Ok(Ok(tcp)) => {
                tcp.set_nodelay(true)?;
                let connector = TlsConnector::from(config.clone());
                let tls = tokio::time::timeout(handshake_timeout, connector.connect(server_name(), tcp))
                    .await
                    .map_err(|_| NetError::Timeout)?
                    .map_err(|err| NetError::Tls(err.to_string()))?;
                let (_, connection) = tls.get_ref();
                let exporter = connection
                    .export_keying_material([0u8; 32], PAIRING_EXPORTER_LABEL, None)
                    .map_err(|err| NetError::Tls(err.to_string()))?;
                let info = peer_info(connection, exporter, candidate)?;
                return Ok((TlsStream::Client(tls), info));
            }
            Ok(Err(err)) => last_error = NetError::Io(err),
            Err(_) => last_error = NetError::Timeout,
        }
    }
    Err(last_error)
}

pub async fn accept_tls(tcp: TcpStream, config: Arc<ServerConfig>, handshake_timeout: Duration) -> Result<(SessionStream, PeerInfo), NetError> {
    let remote_addr = tcp.peer_addr()?;
    tcp.set_nodelay(true)?;
    let acceptor = TlsAcceptor::from(config);
    let tls = tokio::time::timeout(handshake_timeout, acceptor.accept(tcp))
        .await
        .map_err(|_| NetError::Timeout)?
        .map_err(|err| NetError::Tls(err.to_string()))?;
    let (_, connection) = tls.get_ref();
    let exporter = connection
        .export_keying_material([0u8; 32], PAIRING_EXPORTER_LABEL, None)
        .map_err(|err| NetError::Tls(err.to_string()))?;
    let info = peer_info(connection, exporter, remote_addr)?;
    Ok((TlsStream::Server(tls), info))
}

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("connection closed")]
    Closed,
    #[error("io: {0}")]
    Io(std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtoError),
}

pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Message, ReadError> {
    let mut header = [0u8; FRAME_HEADER_LEN];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Err(ReadError::Closed),
        Err(err) => return Err(ReadError::Io(err)),
    }
    let len = parse_frame_len(header)?;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            ReadError::Closed
        } else {
            ReadError::Io(err)
        }
    })?;
    Ok(decode_payload(&payload)?)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SendError {
    #[error("connection closed")]
    Closed,
    #[error("peer is not draining the queue")]
    Overloaded,
}

#[derive(Debug, Clone)]
pub struct Outbound {
    high: mpsc::Sender<Message>,
    low: mpsc::Sender<Message>,
}

impl Outbound {
    pub fn send_high(&self, message: Message) -> Result<(), SendError> {
        self.high.try_send(message).map_err(|err| match err {
            mpsc::error::TrySendError::Full(_) => SendError::Overloaded,
            mpsc::error::TrySendError::Closed(_) => SendError::Closed,
        })
    }

    pub async fn send_low(&self, message: Message) -> Result<(), SendError> {
        self.low.send(message).await.map_err(|_| SendError::Closed)
    }

    pub fn is_closed(&self) -> bool {
        self.high.is_closed()
    }
}

fn is_move(message: &Message) -> bool {
    matches!(message, Message::Input(InputEvent::MouseMove { .. }))
}

fn merge_move(into: &mut Message, next: &Message) -> bool {
    match (into, next) {
        (Message::Input(InputEvent::MouseMove { dx, dy }), Message::Input(InputEvent::MouseMove { dx: ndx, dy: ndy })) => {
            let mx = dx.saturating_add(*ndx);
            let my = dy.saturating_add(*ndy);
            if mx.abs() > MOVE_COALESCE_LIMIT || my.abs() > MOVE_COALESCE_LIMIT {
                return false;
            }
            *dx = mx;
            *dy = my;
            true
        }
        _ => false,
    }
}

pub fn spawn_writer<W>(writer: W) -> (Outbound, JoinHandle<Result<(), std::io::Error>>)
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (high_tx, mut high_rx) = mpsc::channel::<Message>(HIGH_PRIORITY_CAPACITY);
    let (low_tx, mut low_rx) = mpsc::channel::<Message>(LOW_PRIORITY_CAPACITY);
    let handle = tokio::spawn(async move {
        let mut writer = BufWriter::with_capacity(64 * 1024, writer);
        let mut pending: Option<Message> = None;
        loop {
            let mut message = match pending.take() {
                Some(message) => message,
                None => tokio::select! {
                    biased;
                    Some(message) = high_rx.recv() => message,
                    Some(message) = low_rx.recv() => message,
                    else => break,
                },
            };
            if is_move(&message) {
                while let Ok(next) = high_rx.try_recv() {
                    if !merge_move(&mut message, &next) {
                        pending = Some(next);
                        break;
                    }
                }
            }
            match encode_frame(&message) {
                Ok(frame) => writer.write_all(&frame).await?,
                Err(err) => tracing::warn!(kind = message.kind(), error = %err, "dropping unencodable outbound message"),
            }
            if pending.is_none() && high_rx.is_empty() && low_rx.is_empty() {
                writer.flush().await?;
            }
        }
        writer.flush().await?;
        writer.shutdown().await
    });
    (Outbound { high: high_tx, low: low_tx }, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writer_and_reader_roundtrip_over_duplex() {
        let (client, mut server) = tokio::io::duplex(1 << 16);
        let (outbound, handle) = spawn_writer(client);
        outbound.send_high(Message::Ping { nonce: 1 }).unwrap();
        outbound.send_low(Message::Release).await.unwrap();
        assert_eq!(read_message(&mut server).await.unwrap(), Message::Ping { nonce: 1 });
        assert_eq!(read_message(&mut server).await.unwrap(), Message::Release);
        drop(outbound);
        handle.await.unwrap().unwrap();
        assert!(matches!(read_message(&mut server).await, Err(ReadError::Closed)));
    }

    #[tokio::test]
    async fn high_priority_preempts_queued_low_priority() {
        let (client, mut server) = tokio::io::duplex(1 << 20);
        let (outbound, handle) = spawn_writer(client);
        for i in 0..4u64 {
            outbound.send_low(Message::ClipboardChunk { transfer_id: i, offset: 0, data: vec![1; 8] }).await.unwrap();
        }
        outbound.send_high(Message::Input(InputEvent::Key { usage: 4, down: true })).unwrap();
        let mut order = Vec::new();
        for _ in 0..5 {
            order.push(read_message(&mut server).await.unwrap().kind());
        }
        assert_eq!(order, vec!["input", "clipboard_chunk", "clipboard_chunk", "clipboard_chunk", "clipboard_chunk"]);
        drop(outbound);
        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn coalesces_consecutive_mouse_moves_without_reordering_other_events() {
        let (client, mut server) = tokio::io::duplex(1 << 16);
        let (outbound, handle) = spawn_writer(client);
        for _ in 0..50 {
            outbound.send_high(Message::Input(InputEvent::MouseMove { dx: 1, dy: -1 })).unwrap();
        }
        outbound.send_high(Message::Input(InputEvent::MouseButton { button: unify_proto::MouseButton::Left, down: true })).unwrap();
        outbound.send_high(Message::Input(InputEvent::MouseMove { dx: 5, dy: 5 })).unwrap();
        drop(outbound);
        handle.await.unwrap().unwrap();
        let mut total = (0, 0);
        let mut saw_button = false;
        loop {
            match read_message(&mut server).await {
                Ok(Message::Input(InputEvent::MouseMove { dx, dy })) => {
                    if saw_button {
                        assert_eq!((dx, dy), (5, 5));
                    } else {
                        total.0 += dx;
                        total.1 += dy;
                    }
                }
                Ok(Message::Input(InputEvent::MouseButton { .. })) => saw_button = true,
                Ok(other) => panic!("unexpected {other:?}"),
                Err(ReadError::Closed) => break,
                Err(err) => panic!("{err}"),
            }
        }
        assert!(saw_button);
        assert_eq!(total, (50, -50));
    }

    #[tokio::test]
    async fn reader_rejects_oversized_frame_header() {
        let (mut client, mut server) = tokio::io::duplex(1024);
        client.write_all(&(u32::MAX).to_be_bytes()).await.unwrap();
        assert!(matches!(read_message(&mut server).await, Err(ReadError::Protocol(ProtoError::FrameTooLarge(_)))));
    }

    #[tokio::test]
    async fn reader_reports_truncated_frame_as_closed() {
        let (mut client, mut server) = tokio::io::duplex(1024);
        client.write_all(&10u32.to_be_bytes()).await.unwrap();
        client.write_all(&[1, 2, 3]).await.unwrap();
        drop(client);
        assert!(matches!(read_message(&mut server).await, Err(ReadError::Closed)));
    }

    #[tokio::test]
    async fn high_queue_overflow_is_reported() {
        let (client, _server) = tokio::io::duplex(16);
        let (outbound, _handle) = spawn_writer(client);
        let mut overloaded = false;
        for i in 0..(HIGH_PRIORITY_CAPACITY as u64 * 4) {
            if outbound.send_high(Message::Ping { nonce: i }) == Err(SendError::Overloaded) {
                overloaded = true;
                break;
            }
        }
        assert!(overloaded);
    }
}
