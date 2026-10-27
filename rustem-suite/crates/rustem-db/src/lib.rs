// ============================================================================
// RUSTEM SUITE — Crate rustem-db
// Único punto de contacto con SurrealDB y gestor de migraciones .surql.
// ============================================================================

//! # Crate `rustem-db`
//! Capa de acceso a datos. Es el **único** punto de contacto con SurrealDB.
//! Proporciona:
//! - [`SurrealStore`]: conexión y operaciones parametrizadas.
//! - [`MigrationRunner`]: aplicación idempotente de migraciones `.surql`.
//! - Traits de repositorio con inyección automática de `tenant_id`.

pub mod connection;
pub mod migration;
pub mod store;

pub use connection::SurrealConnection;
pub use migration::{MigrationResult, MigrationRunner};
pub use store::{SurrealStore, TenantContext};

#[cfg(test)]
mod tests {
    #[test]
    fn db_stub_smoke_test() {
        // Placeholder: el crate compila y expone sus módulos.
        let _ = ();
    }
}
