// ============================================================================
// RUSTEM SUITE — Crate rustem-web (binario principal)
// Servidor Axum: rutas, handlers, middlewares y estáticos.
// ============================================================================

mod handlers;
mod middleware;
mod router;
mod state;

use base64::Engine;
use http_body_util::BodyExt;
use router::AppRouter;
use state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tower::Service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Iniciando Rustem Suite (servidor Axum + Leptos SSR puro)");

    let state = AppState::from_env().await?;
    let app_router = AppRouter::new(state);
    let router = app_router.into_router();
    let router_clone = router.clone();

    // ---------------------------------------------------------------
    // Servidor HTTP en puerto 8080 (con todo el router de Axum)
    // ---------------------------------------------------------------
    let http_addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    tracing::info!("Servidor HTTP escuchando en http://{}", http_addr);

    let http_router = router.clone();
    let _http_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(http_listener, http_router).await {
            tracing::error!("Error en servidor HTTP: {e}");
        }
    });

    // ---------------------------------------------------------------
    // TLS nativo con rustls (100% Rust, cero OpenSSL/C)
    // Puerto 8443 para HTTPS
    // ---------------------------------------------------------------
    let cert_file = std::env::var("TLS_CERT_FILE").unwrap_or_else(|_| "certs/cert.pem".to_string());
    let key_file = std::env::var("TLS_KEY_FILE").unwrap_or_else(|_| "certs/key.pem".to_string());

    let certs = load_certs(&cert_file)?;
    let key = load_private_key(&key_file)?;

    let mut server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    server_config.max_fragment_size = Some(16384);

    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    let https_addr: SocketAddr = "0.0.0.0:8443".parse()?;
    let https_listener = tokio::net::TcpListener::bind(https_addr).await?;
    tracing::info!("Servidor TLS escuchando en https://{}", https_addr);

    // Loop directo (no envuelto en tokio::spawn) para evitar mover router_clone
    loop {
        match https_listener.accept().await {
            Ok((tcp_stream, remote_addr)) => {
                let acceptor = tls_acceptor.clone();
                let router = router_clone.clone();
                tokio::spawn(async move {
                    match acceptor.accept(tcp_stream).await {
                        Ok(tls_stream) => {
                            let io = hyper_util::rt::TokioIo::new(tls_stream);
                            let service = hyper::service::service_fn(
                                |_req: hyper::Request<hyper::body::Incoming>| {
                                    let router = router.clone();
                                    async move {
                                        let mut router = router;
                                        let result: anyhow::Result<
                                            hyper::Response<
                                                http_body_util::Full<axum::body::Bytes>,
                                            >,
                                        > = (async {
                                            let (parts, incoming_body) = _req.into_parts();
                                            let body_bytes = incoming_body
                                                .collect()
                                                .await
                                                .map_err(|e| -> anyhow::Error { e.into() })?
                                                .to_bytes();
                                            let axum_body = axum::body::Body::from(body_bytes);
                                            let axum_req =
                                                http::Request::from_parts(parts, axum_body);

                                            let axum_resp = router
                                                .call(axum_req)
                                                .await
                                                .map_err(|_| anyhow::anyhow!("router error"))?;

                                            let (parts, body) = axum_resp.into_parts();
                                            let body_bytes = body
                                                .collect()
                                                .await
                                                .map_err(|e| -> anyhow::Error { e.into() })?
                                                .to_bytes();
                                            let hyper_body = http_body_util::Full::new(body_bytes);
                                            Ok(http::Response::from_parts(parts, hyper_body))
                                        })
                                        .await;
                                        result
                                    }
                                },
                            );
                            if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                                hyper_util::rt::TokioExecutor::new(),
                            )
                            .serve_connection(io, service)
                            .await
                            {
                                tracing::warn!(
                                    "Error sirviendo conexión TLS desde {}: {e}",
                                    remote_addr
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Error aceptando conexión TLS desde {}: {e}",
                                remote_addr
                            );
                        }
                    }
                });
            }
            Err(e) => {
                tracing::error!("Error aceptando conexión TCP: {e}");
            }
        }
    }
}

/// Carga certificados X.509 desde un archivo PEM mediante parsing simple.
fn load_certs(path: &str) -> anyhow::Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let content = std::fs::read_to_string(path)?;
    let mut certs = Vec::new();
    let mut remaining = content.as_str();
    while let Some(b64) = extract_pem_block(remaining, "CERTIFICATE") {
        remaining = &remaining[b64.len()..];
        let der = base64::engine::general_purpose::STANDARD.decode(b64)?;
        certs.push(rustls::pki_types::CertificateDer::from(der));
    }
    if certs.is_empty() {
        anyhow::bail!("No se encontraron certificados en {path}");
    }
    Ok(certs)
}

/// Carga una clave privada desde un archivo PEM mediante parsing simple.
fn load_private_key(path: &str) -> anyhow::Result<rustls::pki_types::PrivateKeyDer<'static>> {
    let content = std::fs::read_to_string(path)?;
    let remaining = content.as_str();
    for label in ["RSA PRIVATE KEY", "PRIVATE KEY"] {
        if let Some(b64) = extract_pem_block(remaining, label) {
            let der = base64::engine::general_purpose::STANDARD.decode(b64)?;
            // Box::leak para que los datos de la clave vivan para siempre
            let der_static: &'static [u8] = Box::leak(der.into_boxed_slice());
            let pkcs1: rustls::pki_types::PrivatePkcs1KeyDer = From::from(der_static);
            return Ok(rustls::pki_types::PrivateKeyDer::from(pkcs1));
        }
    }
    anyhow::bail!("No se encontró clave privada en {path}")
}

/// Extrae el contenido base64 de un bloque PEM entre -----BEGIN LABEL----- y -----END LABEL-----.
fn extract_pem_block<'a>(content: &'a str, label: &str) -> Option<&'a str> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = content.find(&begin)?;
    let after_begin = &content[start + begin.len()..];
    let end_pos = after_begin.find(&end)?;
    Some(&after_begin[..end_pos])
}
