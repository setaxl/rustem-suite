// ============================================================================
// RUSTEM SUITE — Crate rustem-sales
// Ventas, cotizaciones, pedidos, facturas SENIAT, IGTF, retenciones.
// ============================================================================

//! # Crate `rustem-sales`
//! Ciclo de ventas adaptado a la legislación fiscal venezolana:
//! - Clientes con RIF y condición IVA.
//! - Motor fiscal: IVA, IGTF, tasa BCV.
//! - Cotizaciones, pedidos y facturas.

pub mod fiscal;
pub mod models;

pub use fiscal::{IvaRate, TaxEngine};
pub use models::{Customer, CustomerViewData, DocumentState, Invoice, Order, Quote, VatCondition};

#[cfg(test)]
mod tests {
    #[test]
    fn sales_smoke_test() {
        let _ = ();
    }
}
