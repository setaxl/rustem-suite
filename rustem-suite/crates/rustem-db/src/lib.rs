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
    use super::{MigrationResult, MigrationRunner, TenantContext};

    #[test]
    fn db_modules_are_exported() {
        // Verifica que los tipos públicos del crate se pueden importar y usar.
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
