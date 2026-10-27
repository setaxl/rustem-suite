// ============================================================================
// RUSTEM SUITE — rustem-auth::errors
// Errores del sistema de autenticación.
// ============================================================================

//! Errores de autenticación y sesión.

use thiserror::Error;

/// Resultado tipificado de operaciones de autenticación.
pub type AuthResult<T> = Result<T, AuthError>;

/// Errores de autenticación.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Credenciales inválidas")]
    InvalidCredentials,
    #[error("Usuario no encontrado")]
    UserNotFound,
    #[error("Cuenta bloqueada temporalmente por intentos fallidos")]
    AccountLocked,
    #[error("Error de hash o cifrado: {0}")]
    Hashing(String),
    #[error("Error de token de sesión: {0}")]
    Token(String),
    #[error("Sesión inválida o expirada")]
    InvalidSession,
    #[error("Error de persistencia: {0}")]
    Persistence(String),
    #[error("Error interno: {0}")]
    Internal(#[from] anyhow::Error),
}
