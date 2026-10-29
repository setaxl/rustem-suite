// ============================================================================
// RUSTEM SUITE — rustem-web::state
// Estado compartido de la aplicación Axum.
// ============================================================================

//! # Módulo `state`
//! Estado de la aplicación.

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub base_url: String,
    pub brute_force: Arc<RwLock<brute_force::BruteForceGuard>>,
}

impl AppState {
    pub async fn from_env() -> anyhow::Result<Self> {
        let base_url = std::env::var("RUSTEM_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        Ok(Self {
            base_url,
            brute_force: Arc::new(RwLock::new(brute_force::BruteForceGuard::new())),
        })
    }
}

// Placeholder
pub mod brute_force {
    pub struct BruteForceGuard;
    impl BruteForceGuard {
        pub fn new() -> Self { Self }
    }
}