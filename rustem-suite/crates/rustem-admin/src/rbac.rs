// ============================================================================
// RUSTEM SUITE — rustem-admin::rbac
// Guard de autorización RBAC para rutas Axum.
// ============================================================================

//! # Módulo `rbac`
//! Matriz de roles y permisos para control de acceso.
//! Proporciona [`RbacGuard`] que verifica si un usuario tiene el permiso
// requerido sobre un recurso dado.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::StatusCode;
use tracing::warn;

use crate::models::{Permission, Role};

/// Resultado de una verificación de permiso.
pub enum AccessDecision {
    /// Acceso permitido.
    Allowed,
    /// Acceso denegado (falta el permiso).
    Denied,
}

/// Guard de RBAC. Verifica si un rol tiene un permiso específico.
pub struct RbacGuard {
    roles: Vec<Role>,
    permissions: Vec<Permission>,
}

impl RbacGuard {
    /// Crea un nuevo guard con la lista de roles y permisos del usuario.
    pub fn new(roles: Vec<Role>, permissions: Vec<Permission>) -> Self {
        Self { roles, permissions }
    }

    /// Verifica si el usuario tiene el permiso `(resource, action)`.
    pub fn can(&self, resource: &str, action: &str) -> AccessDecision {
        let has_permission = self
            .permissions
            .iter()
            .any(|p| p.resource == resource && p.action == action);

        if has_permission {
            AccessDecision::Allowed
        } else {
            warn!(
                "Acceso denegado: no hay permiso para {}/{}",
                resource, action
            );
            AccessDecision::Denied
        }
    }

    /// Verifica si el usuario tiene un rol específico.
    pub fn has_role(&self, role_name: &str) -> bool {
        self.roles.iter().any(|r| r.name == role_name)
    }
}

/// Middleware de Axum para proteger rutas con RBAC.
pub async fn rbac_middleware(
    guard: RbacGuard,
    resource: &'static str,
    action: &'static str,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let decision = guard.can(resource, action);
    match decision {
        AccessDecision::Allowed => Ok(next.run(req).await),
        AccessDecision::Denied => Err(StatusCode::FORBIDDEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rbac_allowed() {
        let roles = vec![Role {
            id: "1".to_string(),
            name: "Administrador".to_string(),
            description: None,
        }];
        let permissions = vec![Permission {
            id: "1".to_string(),
            name: "ver facturas".to_string(),
            resource: "sales".to_string(),
            action: "read".to_string(),
        }];
        let guard = RbacGuard::new(roles, permissions);
        assert!(matches!(
            guard.can("sales", "read"),
            AccessDecision::Allowed
        ));
        assert!(guard.has_role("Administrador"));
        assert!(!guard.has_role("Vendedor"));
    }

    #[test]
    fn rbac_denied() {
        let roles = vec![Role {
            id: "1".to_string(),
            name: "Vendedor".to_string(),
            description: None,
        }];
        let permissions = vec![Permission {
            id: "1".to_string(),
            name: "ver facturas".to_string(),
            resource: "sales".to_string(),
            action: "read".to_string(),
        }];
        let guard = RbacGuard::new(roles, permissions);
        assert!(matches!(
            guard.can("accounting", "write"),
            AccessDecision::Denied
        ));
    }

    #[test]
    fn rbac_guard_has_role() {
        let roles = vec![Role {
            id: "1".to_string(),
            name: "Administrador".to_string(),
            description: None,
        }];
        let permissions = vec![];
        let guard = RbacGuard::new(roles, permissions);
        assert!(guard.has_role("Administrador"));
        assert!(!guard.has_role("Contador"));
        assert!(!guard.has_role("Vendedor"));
        assert!(!guard.has_role("Almacenista"));
    }

    #[test]
    fn rbac_guard_empty_roles() {
        let roles = vec![];
        let permissions = vec![Permission {
            id: "1".to_string(),
            name: "ver ventas".to_string(),
            resource: "sales".to_string(),
            action: "read".to_string(),
        }];
        let guard = RbacGuard::new(roles, permissions);
        assert!(!guard.has_role("Vendedor"));
        assert!(!guard.has_role("Administrador"));
        assert!(matches!(
            guard.can("sales", "read"),
            AccessDecision::Allowed
        ));
        assert!(matches!(
            guard.can("accounting", "read"),
            AccessDecision::Denied
        ));
    }

    #[test]
    fn rbac_guard_empty_permissions() {
        let roles = vec![Role {
            id: "1".to_string(),
            name: "Vendedor".to_string(),
            description: None,
        }];
        let permissions = vec![];
        let guard = RbacGuard::new(roles, permissions);
        assert!(!guard.has_role("Vendedor"));
        assert!(matches!(
            guard.can("sales", "read"),
            AccessDecision::Denied
        ));
    }

    #[test]
    fn rbac_guard_case_sensitive() {
        let roles = vec![Role {
            id: "1".to_string(),
            name: "Administrador".to_string(),
            description: None,
        }];
        let permissions = vec![Permission {
            id: "1".to_string(),
            name: "ver ventas".to_string(),
            resource: "sales".to_string(),
            action: "read".to_string(),
        }];
        let guard = RbacGuard::new(roles, permissions);
        // "VENTAS" es diferente de "sales" (case sensitive)
        assert!(matches!(
            guard.can("VENTAS", "read"),
            AccessDecision::Denied
        ));
        // "READ" es diferente de "read"
        assert!(matches!(
            guard.can("sales", "READ"),
            AccessDecision::Denied
        ));
    }
}
