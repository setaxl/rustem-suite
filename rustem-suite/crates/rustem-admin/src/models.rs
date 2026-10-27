// ============================================================================
// RUSTEM SUITE — rustem-admin::models
// Modelos de tenancy, empresas, roles y permisos.
// ============================================================================

//! # Módulo `models`
//! Entidades de administración:
//! - [`Company`]: empresa / tenant del sistema.
//! - [`CompanyUser`]: asociación usuario <-> empresa.
//! - [`Role`]: rol dentro del sistema RBAC.
//! - [`Permission`]: permiso granular sobre un recurso.

use serde::{Deserialize, Serialize};

/// Empresa / Tenant del sistema ERP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    pub rif: String,
    pub tax_regime: String,
    pub is_vat_agent: bool,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

impl Company {
    pub fn new(id: impl Into<String>, name: impl Into<String>, rif: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            rif: rif.into(),
            tax_regime: "VEN-NIF PYME".to_string(),
            is_vat_agent: false,
            address: None,
            phone: None,
            email: None,
        }
    }
}

/// Asociación usuario <-> empresa (multi-tenancy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyUser {
    pub id: String,
    pub user_id: String,
    pub company_id: String,
    pub role: String,
}

/// Rol del sistema RBAC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Permiso granular sobre un recurso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub resource: String,
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_creation() {
        let c = Company::new("c1", "Empresa Demo", "V-12345678-9");
        assert_eq!(c.name, "Empresa Demo");
        assert_eq!(c.rif, "V-12345678-9");
        assert_eq!(c.tax_regime, "VEN-NIF PYME");
        assert!(!c.is_vat_agent);
    }

    #[test]
    fn role_and_permission() {
        let r = Role {
            id: "r1".to_string(),
            name: "Contador".to_string(),
            description: Some("Gestión contable".to_string()),
        };
        assert_eq!(r.name, "Contador");

        let p = Permission {
            id: "p1".to_string(),
            name: "ver asientos".to_string(),
            resource: "accounting".to_string(),
            action: "read".to_string(),
        };
        assert_eq!(p.resource, "accounting");
        assert_eq!(p.action, "read");
    }
}
