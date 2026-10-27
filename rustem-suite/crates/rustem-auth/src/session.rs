// ============================================================================
// RUSTEM SUITE — rustem-auth::session
// Sesiones HTTP: tokens de 256 bits y hash SHA-256 en SurrealDB.
// ============================================================================

//! # Módulo `session`
//! Gestión de sesiones de autenticación:
//! - Generación de tokens aleatorios de 256 bits con `getrandom`.
//! - Almacenamiento exclusivamente del hash SHA-256 del token en SurrealDB.
//! - Estructuras [`Session`] y [`SessionManager`].

use getrandom::fill as getrandom_fill;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use zeroize::Zeroize;

use crate::errors::{AuthError, AuthResult};

/// Duración predeterminada de una sesión: 8 horas (en segundos).
pub const DEFAULT_SESSION_TTL_SECONDS: i64 = 8 * 60 * 60;

/// Una sesión de usuario autenticado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Hash SHA-256 del token de sesión (hexadecimal).
    pub token_hash: String,
    /// ID del usuario asociado.
    pub user_id: String,
    /// ID del tenant/empresa asociado (opcional).
    pub company_id: Option<String>,
    /// Fecha y hora de expiración (UTC).
    pub expires_at: OffsetDateTime,
    /// Dirección IP del cliente.
    pub ip_address: Option<String>,
    /// User-Agent del cliente.
    pub user_agent: Option<String>,
    /// Fecha y hora de creación (UTC).
    pub created_at: OffsetDateTime,
}

/// Token de sesión en texto plano (solo se muestra una vez al crear la sesión).
#[derive(Debug, Clone)]
pub struct SessionToken(pub String);

impl Zeroize for SessionToken {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl SessionToken {
    /// Genera un token de sesión aleatorio de 256 bits (32 bytes) en hexadecimal.
    pub fn generate() -> AuthResult<Self> {
        let mut bytes = [0u8; 32];
        getrandom_fill(&mut bytes)
            .map_err(|e| AuthError::Token(format!("fallo al generar token: {e}")))?;
        Ok(Self(hex::encode(bytes)))
    }

    /// Calcula el hash SHA-256 del token para almacenamiento en SurrealDB.
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.0.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}

impl std::fmt::Display for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No se imprime el token real por seguridad.
        write!(f, "[SessionToken REDACTED]")
    }
}

/// Gestor de sesiones: crea tokens, calcula hashes y verifica expiración.
pub struct SessionManager;

impl SessionManager {
    /// Crea una nueva sesión con un token de 256 bits.
    pub fn create_session(
        user_id: impl Into<String>,
        company_id: Option<impl Into<String>>,
        ip_address: Option<impl Into<String>>,
        user_agent: Option<impl Into<String>>,
        ttl_seconds: i64,
    ) -> AuthResult<(Session, SessionToken)> {
        let token = SessionToken::generate()?;
        let token_hash = token.hash();
        let now = OffsetDateTime::now_utc();
        let expires_at = now
            .checked_add(time::Duration::seconds(ttl_seconds))
            .ok_or(AuthError::Token("TTL inválido".to_string()))?;

        let session = Session {
            token_hash,
            user_id: user_id.into(),
            company_id: company_id.map(|s| s.into()),
            expires_at,
            ip_address: ip_address.map(|s| s.into()),
            user_agent: user_agent.map(|s| s.into()),
            created_at: now,
        };

        Ok((session, token))
    }

    /// Crea una sesión con TTL predeterminado (8 horas).
    pub fn create_session_default(
        user_id: impl Into<String>,
        company_id: Option<impl Into<String>>,
        ip_address: Option<impl Into<String>>,
        user_agent: Option<impl Into<String>>,
    ) -> AuthResult<(Session, SessionToken)> {
        Self::create_session(
            user_id,
            company_id,
            ip_address,
            user_agent,
            DEFAULT_SESSION_TTL_SECONDS,
        )
    }

    /// Verifica si una sesión ha expirado.
    pub fn is_expired(session: &Session) -> bool {
        OffsetDateTime::now_utc() > session.expires_at
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_generate_is_256_bits() {
        let token = SessionToken::generate().unwrap();
        // 32 bytes = 64 caracteres hexadecimales
        assert_eq!(token.0.len(), 64);
        // Verifica que solo contiene caracteres hex
        assert!(token.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_hash_is_sha256() {
        let token = SessionToken::generate().unwrap();
        let hash = token.hash();
        // SHA-256 = 32 bytes = 64 caracteres hex
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        // El hash debe ser determinista para el mismo token
        let token2 = SessionToken(token.0.clone());
        assert_eq!(token.hash(), token2.hash());
    }

    #[test]
    fn create_session_default_ttl() {
        let (session, _token) = SessionManager::create_session_default(
            "user-1",
            Some("company-1"),
            Some("192.168.1.1"),
            Some("test-agent"),
        )
        .unwrap();
        assert_eq!(session.user_id, "user-1");
        assert_eq!(session.company_id.as_deref(), Some("company-1"));
        assert!(session.expires_at > session.created_at);
        let diff = session.expires_at - session.created_at;
        assert_eq!(diff.whole_seconds(), DEFAULT_SESSION_TTL_SECONDS);
    }

    #[test]
    fn is_expired_works() {
        let now = OffsetDateTime::now_utc();
        let mut session = Session {
            token_hash: "abc".to_string(),
            user_id: "u".to_string(),
            company_id: None,
            expires_at: now.checked_add(time::Duration::seconds(3600)).unwrap(),
            ip_address: None,
            user_agent: None,
            created_at: now,
        };
        assert!(!SessionManager::is_expired(&session));

        session.expires_at = now.checked_sub(time::Duration::seconds(1)).unwrap();
        assert!(SessionManager::is_expired(&session));
    }

    #[test]
    fn create_session_custom_ttl() {
        let (session, token) = SessionManager::create_session(
            "user-1",
            Some("company-1"),
            Some("192.168.1.1"),
            Some("test-agent"),
            3600,
        )
        .unwrap();
        assert_eq!(session.user_id, "user-1");
        assert_eq!(session.company_id.as_deref(), Some("company-1"));
        assert_eq!(session.ip_address.as_deref(), Some("192.168.1.1"));
        assert_eq!(session.user_agent.as_deref(), Some("test-agent"));
        let diff = session.expires_at - session.created_at;
        assert_eq!(diff.whole_seconds(), 3600);

        // Verificar que el token tiene 64 caracteres hexadecimales
        assert_eq!(token.0.len(), 64);
        assert!(token.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_token_zeroize() {
        let mut token = SessionToken("test1234".to_string());
        token.zeroize();
        assert_eq!(token.0, "");
    }

    #[test]
    fn session_zeroize() {
        let mut session = Session {
            token_hash: "hash123".to_string(),
            user_id: "user-1".to_string(),
            company_id: Some("company-1".to_string()),
            expires_at: OffsetDateTime::now_utc(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: OffsetDateTime::now_utc(),
        };
        session.zeroize();
        assert_eq!(session.token_hash, "");
        assert_eq!(session.user_id, "");
        assert_eq!(session.company_id, None);
        assert_eq!(session.ip_address, None);
        assert_eq!(session.user_agent, None);
    }
}
