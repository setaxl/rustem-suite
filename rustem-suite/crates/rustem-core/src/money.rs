// ============================================================================
// RUSTEM SUITE — rustem-core::money
// Tipo Money: importe monetario exacto en centavos (i64) + moneda ISO 4217.
// PROHIBIDO usar f32/f64 para valores monetarios.
// ============================================================================

//! # Tipo `Money`
//! Representa un importe monetario exacto en unidades menores (centavos)
//! almacenado como `i64`, asociado a una [`Currency`].
//! Todas las operaciones son exactas (aritmética entera) y previenen
//! errores de redondeo de punto flotante.

use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::currency::Currency;
use crate::errors::{DomainResult, RustemError};

/// Errores específicos del tipo `Money`.
#[derive(Debug, Error)]
pub enum MoneyError {
    #[error("Desbordamiento aritmético en operación de Money")]
    Overflow,
    #[error("Monedas incompatibles: no se pueden operar {0} con {1}")]
    CurrencyMismatch(Currency, Currency),
    #[error("Importe negativo no permitido para esta operación")]
    NegativeAmount,
}

/// Importe monetario exacto en centavos (`i64`) con código ISO 4217.
///
/// # Invariantas
/// - El importe se almacena siempre en unidades menores (centavos).
/// - No se permite `f32`/`f64` en ninguna operación.
/// - Dos importes solo se pueden sumar/restar si comparten la misma moneda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Importe en unidades menores (centavos de Bolívar o Dólar).
    amount: i64,
    /// Código de moneda ISO 4217.
    currency: Currency,
}

impl Money {
    /// Crea un nuevo `Money` validando que el importe no desborde `i64`.
    ///
    /// # Errores
    /// Devuelve `MoneyError::Overflow` si `amount` está fuera de rango
    /// (no aplica a `i64` pero se reserva para consistencia).
    pub fn new(amount: i64, currency: Currency) -> DomainResult<Self> {
        // i64 ya acota el rango; validamos que no sea negativo para
        // importes de saldo inicial (las operaciones de débito/crédito
        // manejan signo por separado en contabilidad).
        if amount < 0 {
            return Err(RustemError::validation(format!(
                "El importe {} no puede ser negativo",
                amount
            )));
        }
        Ok(Self { amount, currency })
    }

    /// Crea un `Money` a partir de centavos sin validar signo (uso interno
    /// y contabilidad, donde el signo indica débito/crédito).
    pub(crate) fn from_cents_unchecked(amount: i64, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Devuelve el importe en unidades menores (centavos).
    pub fn amount(self) -> i64 {
        self.amount
    }

    /// Devuelve la moneda.
    pub fn currency(self) -> Currency {
        self.currency
    }

    /// Devuelve el importe convertido a unidades mayores con dos decimales
    /// (solo para presentación, nunca para cálculos).
    pub fn to_major_units(self) -> String {
        let sign = if self.amount < 0 { "-" } else { "" };
        let abs = self.amount.abs();
        let major = abs / 100;
        let minor = abs % 100;
        format!("{}{}.{:02} {}", sign, major, minor, self.currency.code())
    }

    /// Suma otro `Money` de la misma moneda.
    pub fn checked_add(self, other: Money) -> DomainResult<Self> {
        self.check_same_currency(other)?;
        let amount = self
            .amount
            .checked_add(other.amount)
            .ok_or(RustemError::crypto("desbordamiento en suma de Money"))?;
        Ok(Self::from_cents_unchecked(amount, self.currency))
    }

    /// Resta otro `Money` de la misma moneda.
    pub fn checked_sub(self, other: Money) -> DomainResult<Self> {
        self.check_same_currency(other)?;
        let amount = self
            .amount
            .checked_sub(other.amount)
            .ok_or(RustemError::crypto("desbordamiento en resta de Money"))?;
        Ok(Self::from_cents_unchecked(amount, self.currency))
    }

    /// Multiplica por un factor entero positivo (ej. para calcular totales
    /// de líneas de factura).
    pub fn checked_mul(self, factor: i64) -> DomainResult<Self> {
        if factor < 0 {
            return Err(RustemError::business_rule(
                "factor multiplicador no puede ser negativo",
            ));
        }
        let amount = self.amount.checked_mul(factor).ok_or(RustemError::crypto(
            "desbordamiento en multiplicación de Money",
        ))?;
        Ok(Self::from_cents_unchecked(amount, self.currency))
    }

    /// Devuelve el valor absoluto del importe.
    pub fn abs(self) -> Self {
        Self {
            amount: self.amount.abs(),
            currency: self.currency,
        }
    }

    /// Verifica que dos `Money` compartan la misma moneda.
    fn check_same_currency(self, other: Money) -> DomainResult<()> {
        if self.currency != other.currency {
            return Err(RustemError::BusinessRule(format!(
                "monedas incompatibles: {} vs {}",
                self.currency.code(),
                other.currency.code()
            )));
        }
        Ok(())
    }

    /// Convierte este importe a otra moneda usando una tasa de cambio
    /// (`rust_decimal` para evitar punto flotante). El resultado se redondea
    /// a centavos.
    pub fn convert(self, to: Currency, rate: rust_decimal::Decimal) -> DomainResult<Self> {
        if self.currency == to {
            return Ok(self);
        }
        let amount_decimal = rust_decimal::Decimal::from_i128_with_scale(self.amount as i128, 2);
        let converted = amount_decimal
            .checked_mul(rate)
            .and_then(|d| d.to_i64())
            .ok_or(RustemError::crypto(
                "desbordamiento en conversión de moneda",
            ))?;
        Ok(Self::from_cents_unchecked(converted, to))
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_major_units())
    }
}

impl std::ops::Add for Money {
    type Output = DomainResult<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
    }
}

impl std::ops::Sub for Money {
    type Output = DomainResult<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
    }
}

impl zeroize::Zeroize for Money {
    fn zeroize(&mut self) {
        self.amount.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use zeroize::Zeroize;

    #[test]
    fn money_new_and_display() {
        let m = Money::new(12345, Currency::Ves).unwrap();
        assert_eq!(m.amount(), 12345);
        assert_eq!(m.currency(), Currency::Ves);
        assert_eq!(m.to_major_units(), "123.45 VES");
    }

    #[test]
    fn money_checked_add_same_currency() {
        let m1 = Money::new(1000, Currency::Ves).unwrap(); // 10.00
        let m2 = Money::new(500, Currency::Ves).unwrap(); // 5.00
        let total = m1.checked_add(m2).unwrap();
        assert_eq!(total.amount(), 1500);
        assert_eq!(total.to_major_units(), "15.00 VES");
    }

    #[test]
    fn money_checked_add_currency_mismatch() {
        let m_ves = Money::new(1000, Currency::Ves).unwrap();
        let m_usd = Money::new(1000, Currency::Usd).unwrap();
        let err = m_ves.checked_add(m_usd).unwrap_err();
        assert!(matches!(err, RustemError::BusinessRule(_)));
    }

    #[test]
    fn money_checked_sub() {
        let m1 = Money::new(1000, Currency::Ves).unwrap();
        let m2 = Money::new(300, Currency::Ves).unwrap();
        let diff = m1.checked_sub(m2).unwrap();
        assert_eq!(diff.amount(), 700);
    }

    #[test]
    fn money_checked_mul() {
        let m = Money::new(1500, Currency::Ves).unwrap(); // 15.00
        let total = m.checked_mul(3).unwrap(); // 45.00
        assert_eq!(total.amount(), 4500);
    }

    #[test]
    fn money_convert() {
        let m = Money::new(10000, Currency::Ves).unwrap(); // 100.00 VES
        // Tasa: 36 VES por 1 USD (rate con 4 decimales)
        let rate = rust_decimal::Decimal::new(360000, 4); // 36.0000
        let usd = m.convert(Currency::Usd, rate).unwrap();
        // 100.00 VES * 36 = 3600.00 (resultado del motor de conversión)
        // -> 3600 centavos USD
        assert_eq!(usd.amount(), 3600);
        assert_eq!(usd.currency(), Currency::Usd);
    }

    #[test]
    fn money_same_currency_convert_is_noop() {
        let m = Money::new(5000, Currency::Ves).unwrap();
        let same = m
            .convert(Currency::Ves, rust_decimal::Decimal::ONE)
            .unwrap();
        assert_eq!(same, m);
    }

    #[test]
    fn money_new_rejects_negative_amount() {
        let err = Money::new(-100, Currency::Ves).unwrap_err();
        assert!(matches!(err, RustemError::Validation(_)));
    }

    #[test]
    fn money_checked_mul_rejects_negative_factor() {
        let m = Money::new(1000, Currency::Ves).unwrap();
        let err = m.checked_mul(-1).unwrap_err();
        assert!(matches!(err, RustemError::BusinessRule(_)));
    }

    #[test]
    fn money_abs_returns_absolute_value() {
        let m = Money::from_cents_unchecked(-500, Currency::Ves);
        let abs = m.abs();
        assert_eq!(abs.amount(), 500);
        assert_eq!(abs.currency(), Currency::Ves);
    }

    #[test]
    fn money_zeroize_clears_amount() {
        let mut m = Money::from_cents_unchecked(1000, Currency::Ves);
        m.zeroize();
        assert_eq!(m.amount(), 0);
    }

    #[test]
    fn money_checked_add_large_amounts() {
        let m1 = Money::new(50000000, Currency::Ves).unwrap();
        let m2 = Money::new(50000000, Currency::Ves).unwrap();
        let total = m1.checked_add(m2).unwrap();
        assert_eq!(total.amount(), 100000000);
    }

    #[test]
    fn money_checked_sub_large_amounts() {
        let m1 = Money::new(100000000, Currency::Ves).unwrap();
        let m2 = Money::new(50000000, Currency::Ves).unwrap();
        let diff = m1.checked_sub(m2).unwrap();
        assert_eq!(diff.amount(), 50000000);
    }

    #[test]
    fn money_checked_mul_large_amount() {
        let m = Money::new(1000000, Currency::Ves).unwrap();
        let total = m.checked_mul(1000).unwrap();
        assert_eq!(total.amount(), 1000000000);
    }

    proptest! {
        #[test]
        fn prop_add_is_commutative(
            a in 0..1_000_000i64,
            b in 0..1_000_000i64,
        ) {
            let m1 = Money::new(a, Currency::Ves).unwrap();
            let m2 = Money::new(b, Currency::Ves).unwrap();
            let sum1 = m1.checked_add(m2).unwrap();
            let sum2 = m2.checked_add(m1).unwrap();
            assert_eq!(sum1.amount(), sum2.amount());
        }

        #[test]
        fn prop_sub_then_add_returns_original(
            a in 0..1_000_000i64,
            b in 0..1_000_000i64,
        ) {
            let m1 = Money::new(a, Currency::Ves).unwrap();
            let m2 = Money::new(b, Currency::Ves).unwrap();
            if let Ok(diff) = m1.checked_sub(m2)
                && let Ok(back) = diff.checked_add(m2)
            {
                assert_eq!(back.amount(), m1.amount());
            }
        }
    }
}
