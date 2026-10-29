// ============================================================================
// RUSTEM SUITE — rustem_db
// Único punto de contacto con SurrealDB y gestor de migraciones .surql.
// ============================================================================

//! # Crate `rustem_db`
//! Capa de acceso a datos. Es el **unico** punto de contacto con SurrealDB.
//! Proporciona:
//! - [`SurrealStore`]: conexion y operaciones parametrizadas.
//! - [`MigrationRunner`]: aplicacion idempotente de migraciones `.surql`.
//! - Traits de repositorio con inyeccion automatica de `tenant_id`.

// ---------------------------------------------------------------------------
// Modulos
// ---------------------------------------------------------------------------

pub mod connection;
pub mod store;
pub mod migration;

// Reexportacion de tipos principales.
pub use connection::{DbConnection, DbResponse, SurrealConnection};
pub use store::{params_to_value, SurrealStore, SurrealStoreImpl, TenantContext};
pub use migration::{MigrationResult, MigrationRunner};

// ---------------------------------------------------------------------------
// Pruebas unitarias del crate
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_modules_are_exported() {
        // Verifica que los tipos publicos del crate se pueden importar y usar.
        let _ctx = TenantContext::new("company-1");
        let _runner = MigrationRunner::new();
        let _result = MigrationResult {
            applied: 0,
            skipped: 0,
        };
        assert_eq!(_ctx.tenant_id, "company-1");
        assert_eq!(_result.applied, 0);
        assert_eq!(_result.skipped, 0);
    }

    #[test]
    fn tenant_context_has_expected_fields() {
        let ctx = TenantContext::new("tenant-abc");
        assert_eq!(ctx.tenant_id, "tenant-abc");
        assert!(ctx.user_id.is_none());
        assert!(ctx.roles.is_empty());
        assert!(ctx.permissions.is_empty());
        assert!(ctx.ip_address.is_none());
        assert!(ctx.user_agent.is_none());
    }

    #[test]
    fn tenant_context_has_permission_works() {
        let mut ctx = TenantContext::new("tenant-1");
        ctx.permissions.push("sales:read".to_string());
        ctx.permissions.push("sales:write".to_string());
        assert!(ctx.has_permission("sales", "read"));
        assert!(ctx.has_permission("sales", "write"));
        assert!(!ctx.has_permission("accounting", "read"));
        assert!(!ctx.has_role("Contador"));
    }

    #[test]
    fn tenant_context_has_role_works() {
        let mut ctx = TenantContext::new("tenant-1");
        ctx.roles.push("Contador".to_string());
        assert!(ctx.has_role("Contador"));
        assert!(!ctx.has_role("Vendedor"));
    }
}