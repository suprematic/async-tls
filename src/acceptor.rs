use crate::common::tls_state::TlsState;
use crate::server;

use futures_io::{AsyncRead, AsyncWrite};
use rustls::{ServerConfig, ServerConnection /*ServerSession, Session*/};
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// The TLS accepting part. The acceptor drives
/// the server side of the TLS handshake process. It works
/// on any asynchronous stream.
///
/// It provides a simple interface (`accept`), returning a future
/// that will resolve when the handshake process completed. On
/// success, it will hand you an async `TLSStream`.
///
/// ## Example
///
/// See /examples/server for an example.
#[derive(Clone)]
pub struct TlsAcceptor {
    inner: Arc<ServerConfig>,
}

impl TlsAcceptor {
    /// Accept a client connections. `stream` can be any type implementing `AsyncRead` and `AsyncWrite`,
    /// such as TcpStreams or Unix domain sockets.
    ///
    /// Otherwise, it will return a `Accept` Future, representing the Acceptance part of a
    /// Tls handshake. It will resolve when the handshake is over.
    #[inline]
    pub fn accept<IO>(&self, stream: IO) -> Accept<IO>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
    {
        self.accept_with(stream, |_| (), None)
    }

    pub fn accept_opt<IO>(&self, stream: IO, buffer_limit: usize) -> Accept<IO>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
    {
        self.accept_with(stream, |_| (), Some(buffer_limit))
    }

    // Currently private, as exposing ServerConnections exposes rusttls
    fn accept_with<IO, F>(&self, stream: IO, f: F, buffer_limit: Option<usize>) -> Accept<IO>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
        F: FnOnce(&mut ServerConnection),
    {
        let mut conn = match ServerConnection::new(self.inner.clone()) {
            Ok(conn) => conn,
            Err(_) => {
                return Accept(server::MidHandshake::End);
            }
        };
        conn.set_buffer_limit(buffer_limit);
        f(&mut conn);

        Accept(server::MidHandshake::Handshaking(server::TlsStream {
            conn,
            io: stream,
            state: TlsState::Stream,
        }))
    }
}

/// Future returned from `TlsAcceptor::accept` which will resolve
/// once the accept handshake has finished.
pub struct Accept<IO>(server::MidHandshake<IO>);

impl<IO: AsyncRead + AsyncWrite + Unpin> Future for Accept<IO> {
    type Output = io::Result<server::TlsStream<IO>>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl From<Arc<ServerConfig>> for TlsAcceptor {
    fn from(inner: Arc<ServerConfig>) -> TlsAcceptor {
        TlsAcceptor { inner }
    }
}

impl From<ServerConfig> for TlsAcceptor {
    fn from(inner: ServerConfig) -> TlsAcceptor {
        TlsAcceptor {
            inner: Arc::new(inner),
        }
    }
}
