// ============================================================================
// RUSTEM SUITE — Crate rustem-admin
// Tenancy, roles, permisos (RBAC), usuarios y empresas.
// ============================================================================

//! # Crate `rustem-admin`
//! Administración de multi-tenancy y matriz RBAC:
//! - Empresas (tenants) y asociación usuario-empresa.
//! - Roles y permisos (matriz en grafo de SurrealDB).
//! - Guards de autorización para rutas Axum.

pub mod models;
pub mod rbac;

pub use models::{Company, CompanyUser, Permission, Role};
pub use rbac::RbacGuard;

#[cfg(test)]
mod tests {
    #[test]
    fn admin_stub_smoke_test() {
        let _ = ();
    }
}
