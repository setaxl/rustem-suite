// ============================================================================
// RUSTEM SUITE — rustem-sales::fiscal
// Motor fiscal venezolano: IVA, IGTF, tasa BCV.
// ============================================================================

use crate::models::Invoice;

/// Alícuotas de IVA vigentes en Venezuela (SENIAT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IvaRate {
    General,  // 16%
    Reducida, // 8%
    Exento,   // 0%
}

impl IvaRate {
    pub fn percentage(self) -> u8 {
        match self {
            IvaRate::General => 16,
            IvaRate::Reducida => 8,
            IvaRate::Exento => 0,
        }
    }
}

/// Motor de cálculo fiscal.
pub struct TaxEngine;

impl TaxEngine {
    /// Calcula el IVA sobre un importe en centavos usando una alícuota.
    /// Resultado en centavos (entero exacto).
    pub fn calculate_iva(amount_cents: i64, rate: IvaRate) -> i64 {
        let pct = rate.percentage() as i64;
        (amount_cents * pct) / 100
    }

    /// Calcula el IGTF (Impuesto Grandes Transacciones Financieras)
    /// sobre transacciones en moneda extranjera.
    /// Resultado en centavos.
    pub fn calculate_igtf(amount_cents: i64, rate_percent: u8) -> i64 {
        (amount_cents * rate_percent as i64) / 100
    }

    /// Aplica los cálculos fiscales a una factura (in place).
    pub fn apply_to_invoice(
        invoice: &mut Invoice,
        subtotal_cents: i64,
        iva_rate: IvaRate,
        igtf_rate_percent: u8,
    ) {
        let iva = Self::calculate_iva(subtotal_cents, iva_rate);
        let igtf = Self::calculate_igtf(subtotal_cents, igtf_rate_percent);
        invoice.subtotal_cents = subtotal_cents;
        invoice.iva_cents = iva;
        invoice.igtf_cents = igtf;
        invoice.total_cents = subtotal_cents + iva + igtf;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iva_general_16_percent() {
        // 100.00 VES = 10000 centavos, IVA 16% = 1600 centavos
        assert_eq!(TaxEngine::calculate_iva(10000, IvaRate::General), 1600);
    }

    #[test]
    fn iva_reducida_8_percent() {
        assert_eq!(TaxEngine::calculate_iva(10000, IvaRate::Reducida), 800);
    }

    #[test]
    fn iva_exento() {
        assert_eq!(TaxEngine::calculate_iva(10000, IvaRate::Exento), 0);
    }

    #[test]
    fn igtf_calculation() {
        // IGTF 3% sobre 100.00 USD = 300 centavos
        assert_eq!(TaxEngine::calculate_igtf(10000, 3), 300);
    }

    #[test]
    fn apply_to_invoice_updates_totals() {
        let mut inv = Invoice {
            id: "f1".to_string(),
            order_id: None,
            customer_id: "c1".to_string(),
            series: "001".to_string(),
            number: 1,
            control_number: "001-000001".to_string(),
            state: crate::models::DocumentState::Borrador,
            subtotal_cents: 0,
            iva_cents: 0,
            igtf_cents: 0,
            total_cents: 0,
            currency: "VES".to_string(),
            exchange_rate: None,
        };
        TaxEngine::apply_to_invoice(&mut inv, 10000, IvaRate::General, 0);
        assert_eq!(inv.subtotal_cents, 10000);
        assert_eq!(inv.iva_cents, 1600);
        assert_eq!(inv.igtf_cents, 0);
        assert_eq!(inv.total_cents, 11600);
    }
}
