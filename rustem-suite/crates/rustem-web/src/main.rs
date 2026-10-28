// ============================================================================
// RUSTEM SUITE — Crate rustem-web (binario principal)
// Servidor Axum: rutas, handlers, middlewares y estáticos.
// ============================================================================

mod handlers;
mod middleware;
mod router;
mod state;

use axum::Router;
use router::AppRouter;
use state::AppState;
use std::net::SocketAddr;

/// Construye el router de la aplicación a partir del estado dado.
/// Útil para pruebas unitarias sin necesidad de iniciar el servidor HTTP.
pub fn create_app(state: AppState) -> Router {
    let app_router = AppRouter::new(state);
    app_router.into_router()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Iniciando Rustem Suite (servidor Axum + Leptos SSR puro)");

    let state = AppState::from_env().await?;
    let router = create_app(state);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Servidor HTTP escuchando en http://{}", addr);

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_app_builds_router() {
        let state = AppState::for_tests();
        let router = create_app(state);
        // Si no panica, el router se construyó correctamente
        let _ = router;
    }
}