// ============================================================================
// RUSTEM SUITE — Crate rustem-web (binario principal)
// Servidor Axum: rutas, handlers, middlewares y estáticos.
// ============================================================================

mod handlers;
mod middleware;
mod router;
mod state;

use router::AppRouter;
use state::AppState;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Iniciando Rustem Suite (servidor Axum + Leptos SSR puro)");

    let state = AppState::from_env().await?;
    let app_router = AppRouter::new(state);
    let router = app_router.into_router();

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Servidor HTTP escuchando en http://{}", addr);

    axum::serve(listener, router).await?;

    Ok(())
}