// ============================================================================
// RUSTEM SUITE — Crate rustem-auth
// Autenticación, Argon2id, sesiones y tokens seguros (256 bits).
// ============================================================================

//! # Crate `rustem-auth`
//! Sistema de autenticación propio:
//! - Hashing de contraseñas con Argon2id (parámetros OWASP).
//! - Generación de tokens de sesión de 256 bits (`getrandom`).
//! - Almacenamiento exclusivamente del hash SHA-256 del token en SurrealDB.
//! - Cookies httpOnly, Secure, SameSite=Lax.
//! - Tipos de solicitud y respuesta para flujo PRG.

pub mod errors;
pub mod handlers;
pub mod password;
pub mod session;

pub use errors::AuthError;
pub use handlers::{AuthResponse, FlashMessage, LoginRequest, RegisterRequest};
pub use password::PasswordHasher;
pub use session::{Session, SessionManager, SessionToken};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_crate_smoke_test() {
        // Prueba de humo: los módulos de auth se cargan correctamente.
        let hasher = PasswordHasher::new();
        let _ = hasher;
        let mgr = SessionManager::default();
        let _ = mgr;
        assert!(true);
    }
}
