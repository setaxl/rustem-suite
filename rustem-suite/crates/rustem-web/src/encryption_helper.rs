// ============================================================================
// RUSTEM SUITE — rustem-web::encryption_helper
// Utilidades para cifrado en reposo de campos sensibles en handlers.
// ============================================================================

//! # Módulo `encryption_helper`
//! Proporciona funciones helper para cifrar y descifrar campos sensibles
//! en datos JSON antes de enviarlos a SurrealDB y al leerlos.
//!
//! Uso típico:
//! ```rust
//! use rustem_web::encryption_helper::{encrypt_json_field, decrypt_json_field};
//! use rustem_core::encryption::MasterKey;
//!
//! let master_key = MasterKey::generate().unwrap();
//! let mut json = serde_json::json!({"password_hash": "mi-hash-plano"});
//! encrypt_json_field(&mut json, "password_hash", &master_key).unwrap();
//! decrypt_json_field(&mut json, "password_hash", &master_key).unwrap();
//! ```

use rustem_core::encryption::{decrypt_json_field, encrypt_json_field, MasterKey};
use serde_json::Value;

/// Cifra un campo específico en un objeto JSON antes de almacenarlo en SurrealDB.
///
/// El campo debe existir y ser una cadena. Se reemplaza con el valor cifrado
/// codificado en hexadecimal.
///
/// # Errores
/// Devuelve un error si falla el cifrado.
pub fn encrypt_sensitive_field(
    data: &mut Value,
    field_name: &str,
    master_key: &MasterKey,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    encrypt_json_field(data, field_name, master_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Descifra un campo cifrado en un objeto JSON al leerlo de SurrealDB.
///
/// Si el campo no existe o no tiene formato de cifrado, no se modifica nada.
///
/// # Errores
/// Devuelve un error si falla el descifrado.
pub fn decrypt_sensitive_field(
    data: &mut Value,
    field_name: &str,
    master_key: &MasterKey,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    decrypt_json_field(data, field_name, master_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_json_field() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = serde_json::json!({
            "id": "user-1",
            "password_hash": "argon2-hash-plano",
            "email": "user@example.com",
        });

        encrypt_sensitive_field(&mut json, "password_hash", &master_key).unwrap();
        assert_ne!(
            json.get("password_hash").unwrap().as_str().unwrap(),
            "argon2-hash-plano"
        );

        decrypt_sensitive_field(&mut json, "password_hash", &master_key).unwrap();
        assert_eq!(
            json.get("password_hash").unwrap(),
            "argon2-hash-plano"
        );
    }
}