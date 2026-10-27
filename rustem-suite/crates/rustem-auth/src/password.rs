// ============================================================================
// RUSTEM SUITE — rustem-auth::password
// Hashing de contraseñas con Argon2id (estándar OWASP).
// ============================================================================

//! # Módulo `password`
//! Hashing y verificación de contraseñas usando Argon2id, el algoritmo
//! recomendado por OWASP para almacenamiento de contraseñas.
//! Utiliza [`zeroize`] para limpiar claves sensibles de la memoria.

use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

use crate::errors::{AuthError, AuthResult};

// Re-exportamos el trait PasswordHasher de argon2 con alias para evitar
// conflicto con nuestro struct PasswordHasher.
use argon2::PasswordHasher as Argon2PasswordHasher;

/// Configuración de parámetros Argon2id conforme a OWASP 2026.
/// - m_cost: 64 MiB de memoria
/// - t_cost: 3 iteraciones
/// - p_cost: 4 hilos de paralelismo
const ARGON2_M_COST: u32 = 65536;
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 4;

/// Hasher de contraseñas con Argon2id.
pub struct PasswordHasher {
    inner: Argon2<'static>,
}

impl PasswordHasher {
    /// Crea un nuevo hasher con parámetros OWASP.
    pub fn new() -> Self {
        let inner = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                ARGON2_M_COST,
                ARGON2_T_COST,
                ARGON2_P_COST,
                Some(16), // longitud de clave derivada
            )
            .expect("Parámetros Argon2 inválidos"),
        );
        Self { inner }
    }

    /// Hash de una contraseña en texto plano.
    /// Devuelve el hash codificado como cadena PHC (Portable Hash Coding).
    ///
    /// # Errores
    /// Devuelve `AuthError::Hashing` si falla el generador de sal o el hash.
    pub fn hash(&self, password: &SecretString) -> AuthResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self
            .inner
            .hash_password(password.expose_secret().as_bytes(), &salt)
            .map_err(|e| AuthError::Hashing(format!("fallo al hashear contraseña: {e}")))?;
        Ok(password_hash.to_string())
    }

    /// Verifica una contraseña en texto plano contra un hash PHC almacenado.
    ///
    /// # Errores
    /// Devuelve `AuthError::InvalidCredentials` si la contraseña no coincide.
    /// Devuelve `AuthError::Hashing` si el hash almacenado tiene formato inválido.
    pub fn verify(&self, password: &SecretString, encoded_hash: &str) -> AuthResult<bool> {
        let parsed_hash = PasswordHash::new(encoded_hash)
            .map_err(|e| AuthError::Hashing(format!("hash almacenado inválido: {e}")))?;
        let result = self
            .inner
            .verify_password(password.expose_secret().as_bytes(), &parsed_hash);
        match result {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for PasswordHasher {
    fn zeroize(&mut self) {
        // Argon2 no expone claves directamente; se descarta la instancia.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn hash_and_verify_correct_password() {
        let hasher = PasswordHasher::new();
        let password = SecretString::new("ContraseñaSegura123!".to_string());
        let hash = hasher.hash(&password).unwrap();
        assert!(!hash.is_empty());
        assert!(hasher.verify(&password, &hash).unwrap());
    }

    #[test]
    fn verify_wrong_password_returns_false() {
        let hasher = PasswordHasher::new();
        let password = SecretString::new("correcta".to_string());
        let wrong = SecretString::new("incorrecta".to_string());
        let hash = hasher.hash(&password).unwrap();
        assert!(!hasher.verify(&wrong, &hash).unwrap());
    }

    #[test]
    fn verify_invalid_hash_format() {
        let hasher = PasswordHasher::new();
        let password = SecretString::new("test".to_string());
        let err = hasher.verify(&password, "hash-invalido").unwrap_err();
        assert!(matches!(err, AuthError::Hashing(_)));
    }

    #[test]
    fn default_hasher_works() {
        let hasher = PasswordHasher::default();
        let password = SecretString::new("test123".to_string());
        let hash = hasher.hash(&password).unwrap();
        assert!(hasher.verify(&password, &hash).unwrap());
    }
}
