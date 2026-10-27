// ============================================================================
// RUSTEM SUITE — rustem-core::encryption
// Cifrado en reposo para campos sensibles (chacha20poly1305, 100% Rust).
// ============================================================================

//! # Módulo `encryption`
//! Cifrado simétrico de campos sensibles en reposo usando
//! [`chacha20poly1305`] (AEAD, 100% Rust, sin dependencias de C/C++).
//!
//! Los datos sensibles (datos bancarios, tokens de integración, claves) se
//! cifran antes de almacenarse en SurrealDB y se descifran al leerlos.
//! La clave maestra se almacena en un `Vec<u8>` y se limpia de la memoria
//! con [`zeroize::Zeroize`] cuando ya no se necesita.

use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce, aead::Aead};
use rand::Rng;
use serde_json::Value;
use zeroize::Zeroize;

use crate::errors::{DomainResult, RustemError};

/// Longitud de la clave maestra en bytes (256 bits para chacha20poly1305).
const MASTER_KEY_LEN: usize = 32;

/// Longitud del nonce en bytes (96 bits para chacha20poly1305).
const NONCE_LEN: usize = 12;

/// Longitud del tag de autenticación en bytes (128 bits para Poly1305).
const TAG_LEN: usize = 16;

/// Clave maestra para cifrado en reposo.
/// Se limpia de la memoria al ser descartada mediante `Zeroize`.
#[derive(Debug, Clone)]
pub struct MasterKey {
    /// Clave de 256 bits almacenada como bytes (limpiada con zeroize al dropear).
    key: Vec<u8>,
}

impl MasterKey {
    /// Crea una nueva `MasterKey` a partir de una cadena de 64 caracteres hexadecimales
    /// (32 bytes).
    ///
    /// # Errores
    /// Devuelve `RustemError::Crypto` si la cadena no tiene exactamente 64 caracteres
    /// hexadecimales.
    pub fn from_hex(hex_key: &str) -> DomainResult<Self> {
        let bytes = hex::decode(hex_key).map_err(|e| {
            RustemError::crypto(format!("clave maestra inválida (hex decode): {e}"))
        })?;
        if bytes.len() != MASTER_KEY_LEN {
            return Err(RustemError::crypto(format!(
                "clave maestra debe tener {} bytes, tiene {}",
                MASTER_KEY_LEN,
                bytes.len()
            )));
        }
        Ok(Self { key: bytes })
    }

    /// Genera una nueva clave maestra aleatoria de 256 bits.
    ///
    /// # Errores
    /// Devuelve `RustemError::Crypto` si falla el generador de números aleatorios.
    pub fn generate() -> DomainResult<Self> {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; MASTER_KEY_LEN];
        rng.fill(&mut bytes);
        Ok(Self {
            key: bytes.to_vec(),
        })
    }

    /// Devuelve los bytes de la clave (solo para uso interno de cifrado).
    pub fn key_bytes(&self) -> &[u8] {
        &self.key
    }
}

impl Zeroize for MasterKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

/// Cifra un campo sensible para almacenamiento en reposo.
///
/// El resultado es el nonce (12 bytes) concatenado con el ciphertext + tag
/// de autenticación, todo codificado en hexadecimal.
///
/// # Errores
/// Devuelve `RustemError::Crypto` si falla el cifrado.
pub fn encrypt_field(plaintext: &[u8], master_key: &MasterKey) -> DomainResult<String> {
    let key_array: [u8; MASTER_KEY_LEN] = master_key
        .key_bytes()
        .try_into()
        .expect("clave maestra es 32 bytes");
    let key = chacha20poly1305::Key::from(key_array);
    let cipher = ChaCha20Poly1305::new(&key);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rand::thread_rng();
    rng.fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| RustemError::crypto(format!("fallo al cifrar campo: {e}")))?;

    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(hex::encode(result))
}

/// Descifra un campo cifrado con [`encrypt_field`].
///
/// El input debe ser la representación hexadecimal del nonce + ciphertext + tag.
///
/// # Errores
/// Devuelve `RustemError::Crypto` si el formato es inválido o el descifrado
/// falla (tag de autenticación no coincide).
pub fn decrypt_field(encoded: &str, master_key: &MasterKey) -> DomainResult<Vec<u8>> {
    let data = hex::decode(encoded)
        .map_err(|e| RustemError::crypto(format!("cifrado inválido (hex decode): {e}")))?;

    if data.len() < NONCE_LEN + TAG_LEN {
        return Err(RustemError::crypto(
            "cifrado demasiado corto: falta nonce o tag".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let key_array: [u8; MASTER_KEY_LEN] = master_key
        .key_bytes()
        .try_into()
        .expect("clave maestra es 32 bytes");
    let key = chacha20poly1305::Key::from(key_array);
    let cipher = ChaCha20Poly1305::new(&key);
    let nonce_array: [u8; NONCE_LEN] = nonce_bytes.try_into().expect("nonce es 12 bytes");
    let nonce = Nonce::from(nonce_array);

    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| RustemError::crypto(format!("fallo al descifrar campo: {e}")))
}

/// Cifra un campo específico en un objeto JSON y lo reemplaza con el valor
/// cifrado codificado en hexadecimal.
///
/// Si el campo no existe o no es una cadena, no se modifica nada.
///
/// # Errores
/// Devuelve `RustemError::Crypto` si falla el cifrado.
pub fn encrypt_json_field(
    value: &mut Value,
    field_name: &str,
    master_key: &MasterKey,
) -> DomainResult<()> {
    if let Some(field_value) = value.get_mut(field_name) {
        if let Some(text) = field_value.as_str() {
            let encrypted = encrypt_field(text.as_bytes(), master_key)?;
            *field_value = Value::String(encrypted);
        }
    }
    Ok(())
}

/// Descifra un campo cifrado en un objeto JSON.
///
/// Si el campo no existe o no es una cadena con formato de cifrado
/// (hexadecimal), no se modifica nada.
///
/// # Errores
/// Devuelve `RustemError::Crypto` si falla el descifrado.
pub fn decrypt_json_field(
    value: &mut Value,
    field_name: &str,
    master_key: &MasterKey,
) -> DomainResult<()> {
    if let Some(field_value) = value.get_mut(field_name) {
        if let Some(text) = field_value.as_str() {
            let decrypted = decrypt_field(text, master_key)?;
            let decrypted_text = String::from_utf8(decrypted)
                .map_err(|e| RustemError::crypto(format!("texto descifrado inválido: {e}")))?;
            *field_value = Value::String(decrypted_text);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let master_key = MasterKey::generate().unwrap();
        let plaintext = b"datos bancarios sensibles";
        let encoded = encrypt_field(plaintext, &master_key).unwrap();
        assert_ne!(encoded, hex::encode(plaintext));
        let decrypted = decrypt_field(&encoded, &master_key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let master_key = MasterKey::generate().unwrap();
        let wrong_key = MasterKey::generate().unwrap();
        let plaintext = b"datos sensibles";
        let encoded = encrypt_field(plaintext, &master_key).unwrap();
        let err = decrypt_field(&encoded, &wrong_key).unwrap_err();
        assert!(matches!(err, RustemError::Crypto(_)));
    }

    #[test]
    fn decrypt_tampered_ciphertext_fails() {
        let master_key = MasterKey::generate().unwrap();
        let plaintext = b"datos sensibles";
        let encoded = encrypt_field(plaintext, &master_key).unwrap();
        // Cambiar un byte del ciphertext (después del nonce).
        let mut data = hex::decode(&encoded).unwrap();
        data[NONCE_LEN] ^= 0x01;
        let tampered = hex::encode(data);
        let err = decrypt_field(&tampered, &master_key).unwrap_err();
        assert!(matches!(err, RustemError::Crypto(_)));
    }

    #[test]
    fn master_key_from_hex_validates_length() {
        let key = MasterKey::generate().unwrap();
        let hex_str = hex::encode(&key.key);
        // Clave válida (64 caracteres hex).
        assert!(MasterKey::from_hex(&hex_str).is_ok());
        // Clave demasiado corta.
        assert!(MasterKey::from_hex("deadbeef").is_err());
        // Caracteres no hexadecimales.
        assert!(MasterKey::from_hex(&"Z".repeat(64)).is_err());
    }

    #[test]
    fn encrypt_decrypt_empty_plaintext() {
        let master_key = MasterKey::generate().unwrap();
        let encoded = encrypt_field(&[], &master_key).unwrap();
        let decrypted: Vec<u8> = decrypt_field(&encoded, &master_key).unwrap();
        assert_eq!(decrypted, Vec::<u8>::new());
    }

    #[test]
    fn encrypt_decrypt_json_field_roundtrip() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = Value::Object(serde_json::Map::from_iter([
            ("id".to_string(), Value::String("user-1".to_string())),
            ("password_hash".to_string(), Value::String("argon2-hash-aqui".to_string())),
            ("email".to_string(), Value::String("user@example.com".to_string())),
        ]));

        encrypt_json_field(&mut json, "password_hash", &master_key).unwrap();

        // El campo debe estar cifrado (no es texto plano)
        let encrypted_value = json.get("password_hash").unwrap().as_str().unwrap();
        assert_ne!(encrypted_value, "argon2-hash-aqui");
        assert!(encrypted_value.len() > 20); // hex de nonce + ciphertext + tag

        // Otros campos no deben cambiar
        assert_eq!(json.get("email").unwrap(), "user@example.com");

        // Descifrar
        decrypt_json_field(&mut json, "password_hash", &master_key).unwrap();
        assert_eq!(json.get("password_hash").unwrap(), "argon2-hash-aqui");
    }

    #[test]
    fn encrypt_json_field_missing_field_no_error() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = Value::Object(serde_json::Map::from_iter([
            ("id".to_string(), Value::String("user-1".to_string())),
        ]));

        // No hay campo "password_hash", no debe fallar
        let result = encrypt_json_field(&mut json, "password_hash", &master_key);
        assert!(result.is_ok());
        assert!(!json.as_object().unwrap().contains_key("password_hash"));
    }

    #[test]
    fn decrypt_json_field_missing_field_no_error() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = Value::Object(serde_json::Map::from_iter([
            ("id".to_string(), Value::String("user-1".to_string())),
        ]));

        // No hay campo para descifrar, no debe fallar
        let result = decrypt_json_field(&mut json, "nonexistent", &master_key);
        assert!(result.is_ok());
    }

    #[test]
    fn encrypt_json_field_non_string_value_no_change() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = Value::Object(serde_json::Map::from_iter([
            ("id".to_string(), Value::String("user-1".to_string())),
            ("amount".to_string(), Value::Number(100.into())),
        ]));

        // El campo "amount" es un número, no debe cambiar
        let result = encrypt_json_field(&mut json, "amount", &master_key);
        assert!(result.is_ok());
        assert_eq!(json.get("amount").unwrap(), &Value::Number(100.into()));
    }

    #[test]
    fn decrypt_json_field_non_string_value_no_change() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = Value::Object(serde_json::Map::from_iter([
            ("id".to_string(), Value::String("user-1".to_string())),
            ("amount".to_string(), Value::Number(100.into())),
        ]));

        // El campo "amount" es un número, no debe cambiar
        let result = decrypt_json_field(&mut json, "amount", &master_key);
        assert!(result.is_ok());
        assert_eq!(json.get("amount").unwrap(), &Value::Number(100.into()));
    }

    #[test]
    fn encrypt_decrypt_multiple_fields() {
        let master_key = MasterKey::generate().unwrap();
        let mut json = Value::Object(serde_json::Map::from_iter([
            ("id".to_string(), Value::String("user-1".to_string())),
            ("password_hash".to_string(), Value::String("hash1".to_string())),
            ("email".to_string(), Value::String("user@example.com".to_string())),
            ("phone".to_string(), Value::String("123456789".to_string())),
        ]));

        encrypt_json_field(&mut json, "password_hash", &master_key).unwrap();
        encrypt_json_field(&mut json, "phone", &master_key).unwrap();

        // Ambos campos deben estar cifrados
        assert_ne!(json.get("password_hash").unwrap().as_str().unwrap(), "hash1");
        assert_ne!(json.get("phone").unwrap().as_str().unwrap(), "123456789");

        // email no debe cambiar
        assert_eq!(json.get("email").unwrap(), "user@example.com");

        // Descifrar ambos
        decrypt_json_field(&mut json, "password_hash", &master_key).unwrap();
        decrypt_json_field(&mut json, "phone", &master_key).unwrap();

        assert_eq!(json.get("password_hash").unwrap(), "hash1");
        assert_eq!(json.get("phone").unwrap(), "123456789");
    }
}