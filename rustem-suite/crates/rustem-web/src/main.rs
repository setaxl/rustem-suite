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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Iniciando Rustem Suite (servidor Axum + Leptos SSR)");

    let state = AppState::from_env().await?;

    let router = AppRouter::new(state).into_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Servidor escuchando en http://0.0.0.0:8080");

    axum::serve(listener, router).await?;
    Ok(())
}
