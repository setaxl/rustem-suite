// ============================================================================
// RUSTEM SUITE — rustem_admin
// Control de acceso basado en roles (RBAC) y permisos
// ============================================================================

//! # Módulo `rustem_admin`
//! Control de acceso basado en roles (RBAC) para Rustem Suite:
//! - Gestión de roles y permisos
//! - Verificación de autorización de operaciones
//! - Middleware de protección de rutas

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errores del dominio de administración
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("rol no encontrado: {0}")]
    RoleNotFound(String),
    #[error("permiso denegado: {0}")]
    PermissionDenied(String),
    #[error("usuario no encontrado: {0}")]
    UserNotFound(String),
    #[error("rol ya existe: {0}")]
    RoleExists(String),
    #[error("operación no válida: {0}")]
    InvalidOperation(String),
}

/// Result type for admin operations
pub type AdminResult<T> = Result<T, AdminError>;

// ---------------------------------------------------------------------------
// Role (Rol)
// ---------------------------------------------------------------------------

/// Un rol en el sistema con permisos asociados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Identificador único del rol.
    pub id: String,
    /// Nombre del rol.
    pub name: String,
    /// Descripción del rol.
    pub description: Option<String>,
    /// Permisos asignados al rol.
    pub permissions: Vec<String>,
}

impl Role {
    /// Crea un nuevo rol.
    pub fn new(id: String, name: String, description: Option<String>, permissions: Vec<String>) -> AdminResult<Self> {
        if name.is_empty() {
            return Err(AdminError::InvalidOperation("El nombre del rol no puede estar vacío".to_string()));
        }
        if permissions.is_empty() {
            return Err(AdminError::InvalidOperation("Un rol debe tener al menos un permiso".to_string()));
        }
        Ok(Self {
            id,
            name,
            description,
            permissions,
        })
    }

    /// Agrega un permiso al rol.
    pub fn add_permission(&mut self, permission: &str) {
        if !self.permissions.contains(&permission.to_string()) {
            self.permissions.push(permission.to_string());
        }
    }

    /// Verifica si el rol tiene un permiso específico.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Obtiene todos los permisos.
    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }
}

// ---------------------------------------------------------------------------
// User (Usuario)
// ---------------------------------------------------------------------------

/// Un usuario del sistema con su rol asignado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Identificador único del usuario.
    pub id: String,
    /// Nombre de usuario.
    pub username: String,
    /// Rol asignado.
    pub role: Role,
}

impl User {
    /// Crea un nuevo usuario con un rol.
    pub fn new(id: String, username: String, role: Role) -> AdminResult<Self> {
        if username.is_empty() {
            return Err(AdminError::InvalidOperation("El nombre de usuario no puede estar vacío".to_string()));
        }
        Ok(Self { id, username, role })
    }

    /// Cambia el rol del usuario.
    pub fn set_role(&mut self, new_role: Role) {
        self.role = new_role;
    }

    /// Verifica si el usuario tiene un permiso específico.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.role.has_permission(permission)
    }
}

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_new() {
        let role = Role::new("R001".to_string(), "Administrador".to_string(), Some("Rol con todos los permisos".to_string()), vec!["read".to_string(), "write".to_string(), "delete".to_string()]);
        let role = role.unwrap();
        assert_eq!(role.id, "R001");
        assert_eq!(role.name, "Administrador");
        assert_eq!(role.description, Some("Rol con todos los permisos".to_string()));
        assert_eq!(role.permissions.len(), 3);
    }

    #[test]
    fn role_invalid_empty_name() {
        let result = Role::new("R002".to_string(), "".to_string(), None, vec!["read".to_string()]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "operación no válida: El nombre del rol no puede estar vacío");
    }

    #[test]
    fn role_invalid_empty_permissions() {
        let result = Role::new("R003".to_string(), "Usuario".to_string(), None, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "operación no válida: Un rol debe tener al menos un permiso");
    }

    #[test]
    fn role_add_permission() {
        let mut role = Role::new("R004".to_string(), "Moderador".to_string(), None, vec!["read".to_string()]).unwrap();
        role.add_permission("write");
        assert!(role.has_permission("write"));
        assert!(!role.has_permission("delete"));
    }

    #[test]
    fn role_has_permission() {
        let role = Role::new("R005".to_string(), "Operador".to_string(), None, vec!["read".to_string(), "write".to_string()]).unwrap();
        assert!(role.has_permission("read"));
        assert!(role.has_permission("write"));
        assert!(!role.has_permission("delete"));
    }

    #[test]
    fn user_new() {
        let role = Role::new("R006".to_string(), "Usuario".to_string(), None, vec!["read".to_string()]).unwrap();
        let user = User::new("U001".to_string(), "juan".to_string(), role).unwrap();
        assert_eq!(user.id, "U001");
        assert_eq!(user.username, "juan");
        assert!(user.has_permission("read"));
    }

    #[test]
    fn user_invalid_empty_username() {
        let role = Role::new("R007".to_string(), "Usuario".to_string(), None, vec!["read".to_string()]).unwrap();
        let result = User::new("U002".to_string(), "".to_string(), role);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "operación no válida: El nombre de usuario no puede estar vacío");
    }

    #[test]
    fn user_set_role() {
        let mut role1 = Role::new("R008".to_string(), "Admin".to_string(), None, vec!["read".to_string(), "write".to_string()]).unwrap();
        let role2 = Role::new("R009".to_string(), "Operador".to_string(), None, vec!["read".to_string()]).unwrap();
        let mut user = User::new("U003".to_string(), "maria".to_string(), role1).unwrap();
        user.set_role(role2);
        assert!(user.has_permission("read"));
        assert!(!user.has_permission("write"));
    }
}