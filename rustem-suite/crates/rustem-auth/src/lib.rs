// ============================================================================
// RUSTEM SUITE — rustem_auth::session
// Sesiones HTTP: tokens de 256 bits y hash SHA-256 en SurrealDB.
// ============================================================================

//! # Modulo `session`
//! Gestion de sesiones de autenticacion.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroize;

use getrandom::getrandom as getrandom_fill;

// ---------------------------------------------------------------------------
// Duracion de sesion
// ---------------------------------------------------------------------------

/// Duracion predeterminada de una sesion: 8 horas (en segundos).
pub const DEFAULT_SESSION_TTL_SECONDS: i64 = 8 * 60 * 60;

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// Una sesion de usuario autenticado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Hash SHA-256 del token de sesion (hexadecimal).
    pub token_hash: String,
    /// ID del usuario asociado.
    pub user_id: String,
    /// ID del tenant/empresa asociado (opcional).
    pub company_id: Option<String>,
    /// Fecha y hora de expiracion (UTC) como timestamp Unix.
    pub expires_at: i64,
    /// Direccion IP del cliente.
    pub ip_address: Option<String>,
    /// User-Agent del cliente.
    pub user_agent: Option<String>,
    /// Fecha y hora de creacion (UTC) como timestamp Unix.
    pub created_at: i64,
}

impl Zeroize for Session {
    fn zeroize(&mut self) {
        self.token_hash.zeroize();
        self.user_id.zeroize();
        self.company_id.zeroize();
        self.ip_address.zeroize();
        self.user_agent.zeroize();
    }
}

// ---------------------------------------------------------------------------
// SessionToken
// ---------------------------------------------------------------------------

/// Token de sesion en texto plano (solo se muestra una vez al crear la sesion).
#[derive(Debug, Clone)]
pub struct SessionToken(pub String);

impl Zeroize for SessionToken {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl SessionToken {
    /// Genera un token de sesion aleatorio de 256 bits (32 bytes) en hexadecimal.
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
        write!(f, "[SESSION_TOKEN_REDACTED]")
    }
}

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

/// Gestion de sesiones: creacion, validacion y expiracion.
pub struct SessionManager;

impl SessionManager {
    /// Crea una nueva sesion con TTL personalizado.
    pub fn create_session(
        user_id: String,
        company_id: Option<String>,
        ttl_seconds: i64,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AuthResult<(SessionToken, Session)> {
        let token = SessionToken::generate()?;
        let token_hash = token.hash();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| AuthError::Internal(format!("error de tiempo: {e}")))?
            .as_secs() as i64;
        let expires_at = now + ttl_seconds;

        let session = Session {
            token_hash,
            user_id,
            company_id,
            expires_at,
            ip_address,
            user_agent,
            created_at: now,
        };

        Ok((token, session))
    }

    /// Crea una sesion con TTL por defecto (8 horas).
    pub fn create_session_default(
        user_id: String,
        company_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AuthResult<(SessionToken, Session)> {
        Self::create_session(user_id, company_id, DEFAULT_SESSION_TTL_SECONDS, ip_address, user_agent)
    }

    /// Verifica si una sesion ha expirado.
    pub fn is_expired(session: &Session) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now > session.expires_at
    }
}

// ---------------------------------------------------------------------------
// Errores de autenticacion
// ---------------------------------------------------------------------------

/// Errores de autenticacion.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("error de token: {0}")]
    Token(String),

    #[error("error de criptografia: {0}")]
    Crypto(String),

    #[error("error de persistencia: {0}")]
    Persistence(String),

    #[error("error interno: {0}")]
    Internal(String),
}

pub type AuthResult<T> = Result<T, AuthError>;

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_generate_is_256_bits() {
        let token = SessionToken::generate().unwrap();
        assert_eq!(token.0.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn token_hash_is_sha256() {
        let token = SessionToken::generate().unwrap();
        let hash = token.hash();
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        assert_ne!(token.0, hash);
    }

    #[test]
    fn token_display_is_redacted() {
        let token = SessionToken::generate().unwrap();
        assert_eq!(token.to_string(), "[SESSION_TOKEN_REDACTED]");
    }

    #[test]
    fn session_zeroize() {
        let mut session = Session {
            token_hash: "hash123".to_string(),
            user_id: "user-1".to_string(),
            company_id: Some("company-1".to_string()),
            expires_at: 0,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: 0,
        };
        session.zeroize();
        assert_eq!(session.token_hash, "");
        assert_eq!(session.user_id, "");
        assert_eq!(session.company_id, None);
        assert_eq!(session.ip_address, None);
        assert_eq!(session.user_agent, None);
    }

    #[test]
    fn token_generate_is_unique() {
        let t1 = SessionToken::generate().unwrap();
        let t2 = SessionToken::generate().unwrap();
        assert_ne!(t1.0, t2.0);
    }

    #[test]
    fn session_token_zeroize() {
        let mut token = SessionToken("test1234".to_string());
        token.zeroize();
        assert_eq!(token.0, "");
    }

    #[test]
    fn create_session_default_ttl() {
        let (token, session) = SessionManager::create_session_default(
            "user-1".to_string(),
            Some("company-1".to_string()),
            Some("192.168.1.1".to_string()),
            Some("test-agent".to_string()),
        )
        .unwrap();
        assert_eq!(session.user_id, "user-1");
        assert_eq!(session.company_id, Some("company-1".to_string()));
        assert_eq!(session.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(session.user_agent, Some("test-agent".to_string()));
        assert!(session.expires_at > 0);
    }

    #[test]
    fn session_is_expired() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut session = Session {
            token_hash: "hash".to_string(),
            user_id: "user-1".to_string(),
            company_id: None,
            expires_at: now - 1,
            ip_address: None,
            user_agent: None,
            created_at: now - 2,
        };
        assert!(SessionManager::is_expired(&session));

        let mut session_valid = Session {
            token_hash: "hash".to_string(),
            user_id: "user-1".to_string(),
            company_id: None,
            expires_at: now + 3600,
            ip_address: None,
            user_agent: None,
            created_at: now,
        };
        assert!(!SessionManager::is_expired(&session_valid));
    }
}