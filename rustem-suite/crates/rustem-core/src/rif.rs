// ============================================================================
// RUSTEM SUITE — rustem-core::rif
// Registro de Información Fiscal (RIF) venezolano con dígito verificador.
// Formatos válidos: V-, J-, G-, E-, P- seguidos de 8 dígitos y dígito de
// control separado por guion (ej. V-12345678-9).
// ============================================================================

//! # Tipo `Rif`
//! Representa un RIF venezolano con validación del dígito verificador
//! según el algoritmo del SENIAT.
//! Formato: `<letra>-<8 dígitos>-<dígito de control>`.
//! Letras válidas: V (persona natural), J (persona jurídica), G,
//! E (extranjero), P (persona jurídica especial).

use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::Validate;

use crate::errors::{DomainResult, RustemError};

/// Errores de validación del RIF.
#[derive(Debug, Error)]
pub enum RifError {
    #[error(
        "Formato de RIF inválido: se esperaba V-/J-/G-/E-/P- seguido de 8 dígitos y dígito de control"
    )]
    InvalidFormat,
    #[error("Dígito de control inválido para el RIF {0}")]
    InvalidCheckDigit(String),
    #[error("Letra de RIF no válida: {0} (use V, J, G, E o P)")]
    InvalidPrefix(char),
}

/// Registro de Información Fiscal (RIF) venezolano validado.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rif {
    /// Representación canónica: `V-12345678-9`.
    value: String,
    /// Letra prefijo (V, J, G, E, P).
    prefix: char,
    /// Los 8 dígitos numéricos (sin formato).
    digits: [u8; 8],
    /// Dígito de control.
    check_digit: u8,
}

impl Rif {
    /// Crea y valida un RIF a partir de una cadena.
    ///
    /// # Errores
    /// Devuelve `RifError` si el formato es inválido o el dígito de
    /// control no coincide con el algoritmo del SENIAT.
    pub fn new(value: &str) -> DomainResult<Self> {
        let trimmed = value.trim();
        let parsed = parse_rif(trimmed)?;
        validate_check_digit(&parsed)?;
        Ok(Rif {
            value: parsed.canonical.clone(),
            prefix: parsed.prefix,
            digits: parsed.digits,
            check_digit: parsed.check_digit,
        })
    }

    /// Devuelve la representación canónica del RIF (`V-12345678-9`).
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Devuelve la letra prefijo.
    pub fn prefix(&self) -> char {
        self.prefix
    }

    /// Devuelve los 8 dígitos como array.
    pub fn digits(&self) -> [u8; 8] {
        self.digits
    }

    /// Devuelve el dígito de control.
    pub fn check_digit(&self) -> u8 {
        self.check_digit
    }

    /// Verifica si el prefijo corresponde a una persona natural (V).
    pub fn is_persona_natural(&self) -> bool {
        self.prefix == 'V'
    }

    /// Verifica si el prefijo corresponde a una persona jurídica (J, G, E, P).
    pub fn is_persona_juridica(&self) -> bool {
        matches!(self.prefix, 'J' | 'G' | 'E' | 'P')
    }

    /// Valida el dígito de control usando el algoritmo del SENIAT:
    /// se multiplican los dígitos (de derecha a izquierda) por factores
    /// 2,3,4,5,6,7,8,9 (ciclicamente), se suman, se toma el módulo 11 y
    /// se resta de 11; si el resultado es 10 o 11 se ajusta según el tipo.
    fn compute_check_digit(prefix: char, digits: &[u8; 8]) -> u8 {
        let factors: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
        let mut sum: u32 = 0;
        // Recorrer de derecha a izquierda (dígito 8 al 1).
        for i in (0..8).rev() {
            let factor_index = (7 - i) % factors.len();
            sum += (digits[i] as u32) * (factors[factor_index] as u32);
        }
        let remainder = sum % 11;
        let mut check = 11 - remainder;
        if check >= 10 {
            check = if prefix == 'V' { 9 } else { 0 };
        }
        check as u8
    }
}

impl std::fmt::Display for Rif {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::str::FromStr for Rif {
    type Err = RustemError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Validate for Rif {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        // El RIF ya está validado al construirse; aquí solo confirmamos.
        if self.value.is_empty() {
            return Err(validator::ValidationErrors::new());
        }
        Ok(())
    }
}

/// Estructura intermedia de parsedo del RIF.
struct ParsedRif {
    canonical: String,
    prefix: char,
    digits: [u8; 8],
    check_digit: u8,
}

/// Parsea una cadena de RIF en sus componentes.
fn parse_rif(value: &str) -> Result<ParsedRif, RustemError> {
    // Formato esperado: L-DDDDDDDD-D (12 caracteres con guiones,
    // ej. V-12345678-9)
    let chars: Vec<char> = value.chars().collect();
    if chars.len() != 12 {
        return Err(RustemError::validation(RifError::InvalidFormat.to_string()));
    }

    let prefix = chars[0];
    if chars[1] != '-' || chars[10] != '-' {
        return Err(RustemError::validation(RifError::InvalidFormat.to_string()));
    }

    if !matches!(prefix, 'V' | 'J' | 'G' | 'E' | 'P') {
        return Err(RustemError::validation(
            RifError::InvalidPrefix(prefix).to_string(),
        ));
    }

    let digit_str: String = chars[2..10].iter().collect();
    let digits: [u8; 8] = match digit_str.parse::<u32>() {
        Ok(n) => {
            let s = format!("{:08}", n);
            let mut arr = [0u8; 8];
            for (i, c) in s.chars().enumerate() {
                arr[i] = c.to_digit(10).unwrap() as u8;
            }
            arr
        }
        Err(_) => {
            return Err(RustemError::validation(RifError::InvalidFormat.to_string()));
        }
    };

    let check_digit = match chars[11].to_digit(10) {
        Some(d) => d as u8,
        None => {
            return Err(RustemError::validation(RifError::InvalidFormat.to_string()));
        }
    };

    Ok(ParsedRif {
        canonical: value.to_string(),
        prefix,
        digits,
        check_digit,
    })
}

/// Valida que el dígito de control coincida con el cálculo del SENIAT.
fn validate_check_digit(parsed: &ParsedRif) -> Result<(), RustemError> {
    let expected = Rif::compute_check_digit(parsed.prefix, &parsed.digits);
    if parsed.check_digit != expected {
        return Err(RustemError::validation(
            RifError::InvalidCheckDigit(parsed.canonical.clone()).to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // V-12345678-9 es un RIF de ejemplo válido (dígito de control 9).
    const VALID_RIF_V: &str = "V-12345678-9";
    const VALID_RIF_J: &str = "J-12345678-9";

    #[test]
    fn rif_valid_parses() {
        let rif = Rif::new(VALID_RIF_V).unwrap();
        assert_eq!(rif.as_str(), VALID_RIF_V);
        assert_eq!(rif.prefix(), 'V');
        assert!(rif.is_persona_natural());
        assert!(!rif.is_persona_juridica());
    }

    #[test]
    fn rif_valid_juridica() {
        let rif = Rif::new(VALID_RIF_J).unwrap();
        assert_eq!(rif.prefix(), 'J');
        assert!(!rif.is_persona_natural());
        assert!(rif.is_persona_juridica());
    }

    #[test]
    fn rif_invalid_format() {
        assert!(Rif::new("123456789").is_err());
        assert!(Rif::new("V-12345678").is_err());
        assert!(Rif::new("V-12345678-90").is_err());
        assert!(Rif::new("X-12345678-9").is_err());
    }

    #[test]
    fn rif_invalid_check_digit() {
        // Dígito de control incorrecto a propósito.
        assert!(Rif::new("V-12345678-0").is_err());
    }

    #[test]
    fn rif_display() {
        let rif = Rif::new(VALID_RIF_V).unwrap();
        assert_eq!(rif.to_string(), VALID_RIF_V);
    }

    #[test]
    fn rif_from_str() {
        use std::str::FromStr;
        let rif = Rif::from_str(VALID_RIF_V).unwrap();
        assert_eq!(rif.as_str(), VALID_RIF_V);
    }

    proptest! {
        #[test]
        fn prop_rif_roundtrip(
            prefix in prop_oneof![Just('V'), Just('J'), Just('G'), Just('E'), Just('P')],
            d0 in 0..9u8, d1 in 0..9u8, d2 in 0..9u8, d3 in 0..9u8,
            d4 in 0..9u8, d5 in 0..9u8, d6 in 0..9u8, d7 in 0..9u8,
        ) {
            let digits = [d0,d1,d2,d3,d4,d5,d6,d7];
            let expected = Rif::compute_check_digit(prefix, &digits);
            let rif_str = format!(
                "{prefix}-{d0}{d1}{d2}{d3}{d4}{d5}{d6}{d7}-{expected}",
                prefix = prefix,
                d0 = d0,
                d1 = d1,
                d2 = d2,
                d3 = d3,
                d4 = d4,
                d5 = d5,
                d6 = d6,
                d7 = d7,
                expected = expected,
            );
            let rif = Rif::new(&rif_str).unwrap();
            assert_eq!(rif.check_digit(), expected);
        }
    }
}
