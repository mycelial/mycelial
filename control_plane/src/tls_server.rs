use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use axum::{extract::Request, Router};
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use rustls::ServerConfig;
use tokio::{net::TcpListener, time::sleep};
use tokio_rustls::TlsAcceptor;
use tower_service::Service as TowerService;

use crate::app;

/// Extends axum request
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub common_name: Arc<str>,
    pub addr: SocketAddr,
}

pub async fn serve(
    listen_addr: SocketAddr,
    service: Router,
    app: app::AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let certificate_bundle = app.certificate_bundle();
    let tcp_listener = TcpListener::bind(listen_addr).await?;
    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(pki::Verifier::new(
            certificate_bundle.ca_cert_key.cert.der().clone(),
        )?))
        .with_single_cert(
            vec![certificate_bundle.cert.clone()],
            certificate_bundle.key.serialize_der().try_into()?,
        )?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    loop {
        let (stream, addr) = match tcp_listener.accept().await {
            Ok((stream, addr)) => (stream, addr),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionRefused
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::ConnectionReset
                ) =>
            {
                continue
            }
            Err(_e) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let service = service.clone();
        let tls_acceptor = tls_acceptor.clone();
        tokio::spawn(async move {
            let stream = tokio::select! {
                stream = tls_acceptor.accept(stream) => {
                    match stream {
                        Ok(stream) => stream,
                        Err(e) => {
                            tracing::error!("error during tls handshake connection from {}: {e}", addr);
                            return;
                        }
                    }
                },
                _ = sleep(Duration::from_secs(10)) => {
                    tracing::error!("timeout during tls handshake connection from {}", addr);
                    return
                }
            };

            let (_, server_connection) = stream.get_ref();
            let common_name = match server_connection.peer_certificates() {
                Some([cert, ..]) => match pki::extract_common_name(cert) {
                    Ok(name) => name,
                    Err(e) => {
                        tracing::error!("failed to extract common name from peer certificate: {e}");
                        return;
                    }
                },
                _ => {
                    tracing::error!("peer certificate missing");
                    return;
                }
            };

            let peer_common_name = PeerInfo {
                common_name: Arc::from(common_name),
                addr,
            };
            let hyper_service =
                hyper::service::service_fn(move |mut request: Request<Incoming>| {
                    request.extensions_mut().insert(peer_common_name.clone());
                    service.clone().call(request)
                });

            Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(stream), hyper_service)
                .await
                .ok();
        });
    }
}
