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
//! - Protección anti-fuerza bruta (rate limiter por IP y usuario).
//! - Tipos de solicitud y respuesta para flujo PRG.

pub mod brute_force;
pub mod errors;
pub mod handlers;
pub mod password;
pub mod session;

pub use brute_force::BruteForceGuard;
pub use errors::{AuthError, AuthResult};
pub use handlers::{AuthResponse, FlashMessage, LoginRequest, RegisterRequest};
pub use password::PasswordHasher;
pub use session::{Session, SessionManager, SessionToken};

#[cfg(test)]
mod tests {
    use super::{BruteForceGuard, PasswordHasher, SessionManager};

    #[test]
    fn auth_crate_smoke_test() {
        let hasher = PasswordHasher::new();
        let _ = hasher;
        let mgr = SessionManager;
        let _ = mgr;
        let guard = BruteForceGuard::new();
        let _ = guard;
        let _ = ();
    }
}
