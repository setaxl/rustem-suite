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
    use super::RbacGuard;
    use crate::models::{Company, CompanyUser, Permission, Role};
    use crate::rbac::AccessDecision;

    #[test]
    fn admin_stub_smoke_test() {
        let _ = Company::new("c1", "Empresa Demo", "V-12345678-9");
        let _ = ();
    }

    #[test]
    fn rbac_denies_cross_tenant_resource() {
        // Un vendedor tiene permiso sobre sales pero NO sobre accounting.
        // Debe denegar acceso a recursos de contabilidad.
        let roles = vec![Role {
            id: "r1".to_string(),
            name: "Vendedor".to_string(),
            description: None,
        }];
        let permissions = vec![Permission {
            id: "p1".to_string(),
            name: "ver ventas".to_string(),
            resource: "sales".to_string(),
            action: "read".to_string(),
        }];
        let guard = RbacGuard::new(roles, permissions);

        // Puede leer ventas (tiene el permiso).
        assert!(matches!(
            guard.can("sales", "read"),
            AccessDecision::Allowed
        ));

        // NO puede escribir contabilidad (no tiene el permiso).
        assert!(matches!(
            guard.can("accounting", "write"),
            AccessDecision::Denied
        ));

        // NO puede leer contabilidad (no tiene el permiso).
        assert!(matches!(
            guard.can("accounting", "read"),
            AccessDecision::Denied
        ));
    }

    #[test]
    fn company_user_associates_tenant_and_user() {
        let cu = CompanyUser {
            id: "cu1".to_string(),
            user_id: "u1".to_string(),
            company_id: "c1".to_string(),
            role: "Vendedor".to_string(),
        };
        assert_eq!(cu.company_id, "c1");
        assert_eq!(cu.user_id, "u1");
        assert_eq!(cu.role, "Vendedor");
    }
}
