// ============================================================================
// RUSTEM SUITE — rustem-core::errors
// Tipo de error del dominio con thiserror y alias de Result tipificado.
// ============================================================================

//! Errores del dominio. Cada variante representa una categoría de fallo de
//! negocio o infraestructura que puede propagarse entre capas.

use thiserror::Error;

/// Resultado tipificado del dominio. Todas las operaciones de dominio
/// devuelven `DomainResult<T>` en lugar de `Result<T, anyhow::Error>`.
pub type DomainResult<T> = Result<T, RustemError>;

/// Errores del dominio de Rustem Suite.
#[derive(Debug, Error)]
pub enum RustemError {
    /// Validación de datos de entrada fallida.
    #[error("Validación fallida: {0}")]
    Validation(String),

    /// Recurso no encontrado.
    #[error("Recurso no encontrado: {0}")]
    NotFound(String),

    /// Regla de negocio violada (invariante de dominio).
    #[error("Regla de negocio violada: {0}")]
    BusinessRule(String),

    /// Error de autenticación o autorización.
    #[error("Error de autenticación/autorización: {0}")]
    Auth(String),

    /// Error de persistencia / base de datos.
    #[error("Error de persistencia: {0}")]
    Persistence(String),

    /// Error de configuración del entorno.
    #[error("Error de configuración: {0}")]
    Config(String),

    /// Error criptográfico (hash, cifrado, token).
    #[error("Error criptográfico: {0}")]
    Crypto(String),

    /// Error interno inesperado (envuelve anyhow).
    #[error("Error interno: {0}")]
    Internal(#[from] anyhow::Error),
}

impl RustemError {
    /// Crea un error de validación.
    pub fn validation(message: impl Into<String>) -> RustemError {
        RustemError::Validation(message.into())
    }

    /// Crea un error de recurso no encontrado.
    pub fn not_found(message: impl Into<String>) -> RustemError {
        RustemError::NotFound(message.into())
    }

    /// Crea un error de regla de negocio.
    pub fn business_rule(message: impl Into<String>) -> RustemError {
        RustemError::BusinessRule(message.into())
    }

    /// Crea un error de autenticación.
    pub fn auth(message: impl Into<String>) -> RustemError {
        RustemError::Auth(message.into())
    }

    /// Crea un error de persistencia.
    pub fn persistence(message: impl Into<String>) -> RustemError {
        RustemError::Persistence(message.into())
    }

    /// Crea un error de configuración.
    pub fn config(message: impl Into<String>) -> RustemError {
        RustemError::Config(message.into())
    }

    /// Crea un error criptográfico.
    pub fn crypto(message: impl Into<String>) -> RustemError {
        RustemError::Crypto(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = RustemError::validation("email inválido".to_string());
        assert_eq!(e.to_string(), "Validación fallida: email inválido");

        let e = RustemError::not_found("usuario no existe".to_string());
        assert_eq!(e.to_string(), "Recurso no encontrado: usuario no existe");

        let e = RustemError::business_rule("débitos != créditos".to_string());
        assert_eq!(
            e.to_string(),
            "Regla de negocio violada: débitos != créditos"
        );
    }

    #[test]
    fn error_constructors_return_err() {
        let e = RustemError::auth("credenciales inválidas");
        assert!(matches!(e, RustemError::Auth(_)));
    }
}
