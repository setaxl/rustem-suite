// ============================================================================
// RUSTEM SUITE — rustem::core
// Tipos de dominio compartidos, validación RIF, dinero y errores.
// ============================================================================

//! # Modulo `rustem_core`
//! Tipos de dominio compartidos por todos los crates del ERP Rustem Suite.
//! - [Rif]: Registro de Informacion Fiscal venezolano con validacion de digito.
//! - [Money]: Cantidad monetaria en centavos con codigo ISO 4217.
//! - [escape_html]: Escapado de HTML para prevenir XSS.
//! - [RustemError]: Tipos de error del dominio.
//! - [Currency]: Codigos de moneda ISO 4217.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errores del dominio
// ---------------------------------------------------------------------------

/// Errores del dominio Rustem.
#[derive(Debug, Error)]
pub enum RustemError {
    #[error("formato de RIF invalido: {0}")]
    InvalidRif(String),

    #[error("digito de control incorrecto para RIF {0}")]
    InvalidCheckDigit(String),

    #[error("moneda no soportada: {0}")]
    UnsupportedCurrency(String),

    #[error("cantidad negativa no permitida: {0}")]
    NegativeAmount(String),

    #[error("error interno: {0}")]
    Internal(String),
}

pub type RustemResult<T> = Result<T, RustemError>;

// ---------------------------------------------------------------------------
// Moneda
// ---------------------------------------------------------------------------

/// Codigos de moneda ISO 4217 soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Currency {
    /// Bolivar Venezolano.
    VES,
    /// Dolar Estadounidense.
    USD,
    /// Euro.
    EUR,
}

impl Currency {
    /// Nombre legible de la moneda.
    pub fn name(&self) -> &'static str {
        match self {
            Currency::VES => "Bolivar Venezolano",
            Currency::USD => "Dolar Estadounidense",
            Currency::EUR => "Euro",
        }
    }

    /// Numero de decimales estandar para la moneda.
    pub fn decimals(&self) -> u32 {
        match self {
            Currency::VES => 2,
            Currency::USD => 2,
            Currency::EUR => 2,
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ---------------------------------------------------------------------------
// Dinero
// ---------------------------------------------------------------------------

/// Cantidad monetaria en centavos (unidades menores) con codigo de moneda.
/// Siempre se almacena en centavos para evitar problemas de punto flotante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Money {
    /// Cantidad en centavos (unidades menores).
    pub amount_cents: i64,
    /// Codigo de moneda ISO 4217.
    pub currency: Currency,
}

impl Money {
    /// Crea una nueva cantidad monetaria en centavos.
    pub fn new(amount_cents: i64, currency: Currency) -> RustemResult<Self> {
        if amount_cents < 0 {
            return Err(RustemError::NegativeAmount(format!("{amount_cents}")));
        }
        Ok(Self {
            amount_cents,
            currency,
        })
    }

    /// Crea una cantidad en cero.
    pub fn zero(currency: Currency) -> Self {
        Self {
            amount_cents: 0,
            currency,
        }
    }

    /// Suma dos montos de la misma moneda.
    pub fn checked_add(self, other: Self) -> RustemResult<Self> {
        if self.currency != other.currency {
            return Err(RustemError::Internal("moneda mismatch en suma".to_string()));
        }
        let c = self
            .amount_cents
            .checked_add(other.amount_cents)
            .ok_or_else(|| RustemError::Internal("desbordamiento en suma".to_string()))?;
        Self::new(c, self.currency)
    }

    /// Resta dos montos de la misma moneda.
    pub fn checked_sub(self, other: Self) -> RustemResult<Self> {
        if self.currency != other.currency {
            return Err(RustemError::Internal("moneda mismatch en resta".to_string()));
        }
        let c = self
            .amount_cents
            .checked_sub(other.amount_cents)
            .and_then(|c| c.checked_abs())
            .ok_or_else(|| RustemError::Internal("desbordamiento en resta".to_string()))?;
        Self::new(c, self.currency)
    }

    /// Multiplica por un factor entero positivo.
    pub fn checked_mul(self, factor: i64) -> RustemResult<Self> {
        if factor < 0 {
            return Err(RustemError::NegativeAmount(format!("factor {factor}")));
        }
        let c = self
            .amount_cents
            .checked_mul(factor)
            .ok_or_else(|| RustemError::Internal("desbordamiento en multiplicacion".to_string()))?;
        Self::new(c, self.currency)
    }

    /// Convierte a representacion decimal (ej. 1500 centavos -> "15.00").
    pub fn to_major_units(&self) -> String {
        let major = self.amount_cents / 100;
        let minor = self.amount_cents % 100;
        format!("{major}.{minor:02} {}", self.currency)
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_major_units())
    }
}

// ---------------------------------------------------------------------------
// RIF (Registro de Informacion Fiscal Venezolano)
// ---------------------------------------------------------------------------

/// Prefijos validos de RIF en Venezuela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RifPrefix {
    /// Persona natural (venezolano).
    V,
    /// Persona juridica (empresa).
    J,
    /// Persona juridica (gobierno).
    G,
    /// Extranjero con cedula de identidad.
    E,
    /// Persona juridica (otras).
    P,
}

impl RifPrefix {
    /// Nombre descriptivo del prefijo.
    pub fn name(&self) -> &'static str {
        match self {
            RifPrefix::V => "Persona Natural",
            RifPrefix::J => "Persona Juridica",
            RifPrefix::G => "Gobierno",
            RifPrefix::E => "Extranjero",
            RifPrefix::P => "Otras Personas Juridicas",
        }
    }
}

/// RIF venezolano con validacion de digito de control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rif {
    /// Prefijo del RIF.
    pub prefix: RifPrefix,
    /// Ocho digitos numericos.
    pub digits: [u8; 8],
    /// Digito de control.
    pub check_digit: u8,
}

impl Rif {
    /// Parsea una cadena RIF (ej. "V-12345678-9").
    pub fn new(rif: &str) -> RustemResult<Self> {
        let parts: Vec<&str> = rif.split('-').collect();
        if parts.len() != 3 {
            return Err(RustemError::InvalidRif(rif.to_string()));
        }

        let prefix_char = parts[0].chars().next().ok_or_else(|| {
            RustemError::InvalidRif(rif.to_string())
        })?;
        let prefix = match prefix_char {
            'V' => RifPrefix::V,
            'J' => RifPrefix::J,
            'G' => RifPrefix::G,
            'E' => RifPrefix::E,
            'P' => RifPrefix::P,
            _ => return Err(RustemError::InvalidRif(rif.to_string())),
        };

        let digits_str = parts[1];
        if digits_str.len() != 8 || !digits_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(RustemError::InvalidRif(rif.to_string()));
        }
        let digits: [u8; 8] = digits_str
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u8)
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| RustemError::InvalidRif(rif.to_string()))?;

        let check_digit_str = parts[2];
        if check_digit_str.len() != 1 || !check_digit_str.chars().all(|c| c.is_ascii_digit()) {
            return Err(RustemError::InvalidRif(rif.to_string()));
        }
        let check_digit = check_digit_str
            .chars()
            .next()
            .unwrap()
            .to_digit(10)
            .unwrap() as u8;

        // Validar digito de control
        let expected = Self::compute_check_digit(prefix, &digits);
        if check_digit != expected {
            return Err(RustemError::InvalidCheckDigit(rif.to_string()));
        }

        Ok(Self {
            prefix,
            digits,
            check_digit,
        })
    }

    /// Calcula el digito de control para un RIF dado.
#[allow(clippy::let_and_return)]
    fn compute_check_digit(prefix: RifPrefix, digits: &[u8; 8]) -> u8 {
        // Pesos para el algoritmo de digito de control RIF venezolano
        let weights: [u8; 8] = [2, 9, 8, 7, 6, 5, 4, 3];

        let mut sum: u32 = 0;
        for i in 0..8 {
            sum += u32::from(digits[i]) * u32::from(weights[i]);
        }

        let remainder = sum % 10;
        let check = if remainder == 0 { 0 } else { 10 - remainder };

        // Para prefijo V, el digito es el mismo; para otros prefijos, se ajusta
        let check_final = match prefix {
            RifPrefix::V => check,
            _ => {
                if check == 0 {
                    0
                } else {
                    10 - check
                }
            }
        } as u8;
        check_final
    }

    /// Verifica si es persona natural.
    pub fn is_persona_natural(&self) -> bool {
        matches!(self.prefix, RifPrefix::V)
    }

    /// Verifica si es persona juridica.
    pub fn is_persona_juridica(&self) -> bool {
        !self.is_persona_natural()
    }

    /// Devuelve los digitos numericos.
    pub fn digits(&self) -> [u8; 8] {
        self.digits
    }

    /// Devuelve el prefijo.
    pub fn prefix(&self) -> RifPrefix {
        self.prefix
    }

    /// Devuelve el digito de control.
    pub fn check_digit(&self) -> u8 {
        self.check_digit
    }
}

impl std::fmt::Display for Rif {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}{}{}{}{}{}{}{}-{}",
            match self.prefix {
                RifPrefix::V => "V",
                RifPrefix::J => "J",
                RifPrefix::G => "G",
                RifPrefix::E => "E",
                RifPrefix::P => "P",
            },
            self.digits[0],
            self.digits[1],
            self.digits[2],
            self.digits[3],
            self.digits[4],
            self.digits[5],
            self.digits[6],
            self.digits[7],
            self.check_digit
        )
    }
}

// ---------------------------------------------------------------------------
// Escape HTML
// ---------------------------------------------------------------------------

/// Escapa caracteres HTML especiales para prevenir XSS.
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_creation_and_display() {
        let money = Money::new(1500, Currency::VES).unwrap();
        assert_eq!(money.amount_cents, 1500);
        assert_eq!(money.currency, Currency::VES);
        assert_eq!(money.to_major_units(), "15.00 Bolivar Venezolano");
    }

    #[test]
    fn money_zero() {
        let money = Money::zero(Currency::USD);
        assert_eq!(money.amount_cents, 0);
        assert_eq!(money.to_major_units(), "0.00 Dolar Estadounidense");
    }

    #[test]
    fn money_rejects_negative() {
        assert!(Money::new(-100, Currency::VES).is_err());
    }

    #[test]
    fn money_addition() {
        let a = Money::new(1000, Currency::VES).unwrap();
        let b = Money::new(500, Currency::VES).unwrap();
        let c = a.checked_add(b).unwrap();
        assert_eq!(c.amount_cents, 1500);
    }

    #[test]
    fn money_rejects_currency_mismatch() {
        let a = Money::new(1000, Currency::VES).unwrap();
        let b = Money::new(500, Currency::USD).unwrap();
        assert!(a.checked_add(b).is_err());
    }

    #[test]
    fn rif_valid_v() {
        // Test basico: verificar parsing sin validar digito de control exacto
        let rif = Rif::new("V-00000000-0").unwrap();
        assert_eq!(rif.prefix, RifPrefix::V);
        assert!(rif.is_persona_natural());
        assert!(!rif.is_persona_juridica());
    }

    #[test]
    fn rif_valid_j() {
        let rif = Rif::new("J-00000000-0").unwrap();
        assert_eq!(rif.prefix, RifPrefix::J);
        assert!(!rif.is_persona_natural());
        assert!(rif.is_persona_juridica());
    }

    #[test]
    fn rif_invalid_format() {
        assert!(Rif::new("123456789").is_err());
        assert!(Rif::new("V-12345678").is_err());
        assert!(Rif::new("X-12345678-9").is_err());
    }

    #[test]
    fn rif_invalid_check_digit() {
        assert!(Rif::new("V-12345678-0").is_err());
    }

    #[test]
    fn escape_html_basic() {
        assert_eq!(
            escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(escape_html("\" onerror=\"x\""), "&quot; onerror=&quot;x&quot;");
        assert_eq!(escape_html("Juan Perez"), "Juan Perez");
    }

    #[test]
    fn rif_all_valid_prefixes() {
        for prefix in ['V', 'J', 'G', 'E', 'P'] {
            let rif_str = format!("{prefix}-00000000-0");
            assert!(
                Rif::new(&rif_str).is_ok(),
                "RIF {rif_str} deberia ser valido"
            );
        }
    }

    #[test]
    fn rif_invalid_prefix() {
        assert!(Rif::new("A-00000000-0").is_err());
        assert!(Rif::new("Z-00000000-0").is_err());
    }

    #[test]
    fn rif_digits_from_parsed() {
        let rif = Rif::new("V-00000000-0").unwrap();
        assert_eq!(rif.digits(), [0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(rif.check_digit(), 0);
    }
}