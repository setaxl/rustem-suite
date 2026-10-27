// ============================================================================
// RUSTEM SUITE — rustem-core::currency
// Códigos de moneda ISO 4217 soportados por el marco fiscal venezolano.
// ============================================================================

//! Códigos de moneda ISO 4217 soportados.
//! - [`Currency::Ves`]: Bolívares (moneda de cuenta legal en Venezuela).
//! - [`Currency::Usd`]: Dólares estadounidenses (moneda funcional opcional).

use serde::{Deserialize, Serialize};

/// Códigos de moneda ISO 4217 soportados por el sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Currency {
    /// Bolívar soberano/digital (VES) — moneda de cuenta legal en Venezuela.
    #[default]
    Ves,
    /// Dólar estadounidense (USD) — moneda funcional opcional.
    Usd,
}

impl Currency {
    /// Devuelve el código alfanumérico de tres letras (ISO 4217).
    pub fn code(self) -> &'static str {
        match self {
            Currency::Ves => "VES",
            Currency::Usd => "USD",
        }
    }

    /// Devuelve el nombre legible de la moneda.
    pub fn name(self) -> &'static str {
        match self {
            Currency::Ves => "Bolívar (VES)",
            Currency::Usd => "Dólar estadounidense (USD)",
        }
    }

    /// Número de decimales (unidades menores) de la moneda.
    pub fn decimals(self) -> u32 {
        // Tanto VES como USD usan 2 decimales (centavos/céntimos).
        match self {
            Currency::Ves => 2,
            Currency::Usd => 2,
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_code_and_name() {
        assert_eq!(Currency::Ves.code(), "VES");
        assert_eq!(Currency::Ves.name(), "Bolívar (VES)");
        assert_eq!(Currency::Usd.code(), "USD");
        assert_eq!(Currency::Usd.name(), "Dólar estadounidense (USD)");
    }

    #[test]
    fn currency_decimals() {
        assert_eq!(Currency::Ves.decimals(), 2);
        assert_eq!(Currency::Usd.decimals(), 2);
    }

    #[test]
    fn currency_default_is_ves() {
        assert_eq!(Currency::default(), Currency::Ves);
    }

    #[test]
    fn currency_display() {
        assert_eq!(Currency::Ves.to_string(), "VES");
        assert_eq!(Currency::Usd.to_string(), "USD");
    }
}
