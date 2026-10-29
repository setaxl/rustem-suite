// ============================================================================
// RUSTEM SUITE — rustem-web::router
// Router Axum: rutas y middlewares.
// ============================================================================

//! # Módulo `router`
//! Definición de rutas de la aplicación Axum.

use axum::Router;
use crate::state::AppState;

pub struct AppRouter {
    state: AppState,
}

impl AppRouter {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn into_router(self) -> Router {
        let state = std::sync::Arc::new(self.state);
        Router::new()
            .route("/", axum::routing::get(crate::handlers::root))
            .route("/login", axum::routing::get(crate::handlers::login_get))
            .route("/logout", axum::routing::post(crate::handlers::logout_post))
    }
}

// Placeholder
pub fn dummy() {}