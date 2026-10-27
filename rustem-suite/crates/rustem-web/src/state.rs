// ============================================================================
// RUSTEM SUITE — rustem-web::state
// Estado compartido de la aplicación Axum.
// ============================================================================

//! # Módulo `state`
//! Estado de la aplicación: conexión a SurrealDB, configuración y guard
//! anti-fuerza bruta.

use rustem_auth::brute_force::BruteForceGuard;
use rustem_db::SurrealConnection;
use rustem_db::store::TenantContext;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Estado compartido de la aplicación (se pasa a los handlers de Axum).
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    /// Conexión a SurrealDB (opcional en tests).
    pub db: Option<Arc<RwLock<SurrealConnection>>>,
    /// URL base de la aplicación.
    pub base_url: String,
    /// Guard anti-fuerza bruta para el endpoint de login.
    pub brute_force: Arc<RwLock<BruteForceGuard>>,
}

impl AppState {
    /// Crea un nuevo estado desde las variables de entorno.
    pub async fn from_env() -> anyhow::Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "ws://localhost:8000/rpc".to_string());
        let database_user = std::env::var("DATABASE_USER").unwrap_or_else(|_| "root".to_string());
        let database_pass = std::env::var("DATABASE_PASS").unwrap_or_else(|_| "root".to_string());
        let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "rustem".to_string());
        let database_name = std::env::var("DATABASE_NAME").unwrap_or_else(|_| "suite".to_string());
        let base_url = std::env::var("RUSTEM_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let db = match SurrealConnection::connect(
            &database_url,
            &database_user,
            &database_pass,
            &namespace,
            &database_name,
        )
        .await
        {
            Ok(conn) => {
                conn.apply_migrations().await?;
                Some(Arc::new(RwLock::new(conn)))
            }
            Err(e) => {
                tracing::warn!("No se pudo conectar a SurrealDB: {e}");
                None
            }
        };

        Ok(Self {
            db,
            base_url,
            brute_force: Arc::new(RwLock::new(BruteForceGuard::new())),
        })
    }

    /// Crea un estado vacío para pruebas (sin SurrealDB).
    #[allow(dead_code)]
    pub fn for_tests() -> Self {
        Self {
            db: None,
            base_url: "http://localhost:8080".to_string(),
            brute_force: Arc::new(RwLock::new(BruteForceGuard::new())),
        }
    }

    /// Obtiene el contexto de tenant desde las extensiones de la request.
    #[allow(dead_code)]
    pub fn tenant_context(&self) -> Option<TenantContext> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_for_tests() {
        let state = AppState::for_tests();
        assert!(state.db.is_none());
        assert_eq!(state.base_url, "http://localhost:8080");
        assert!(state.tenant_context().is_none());
        let _ = state.brute_force;
    }
}
