// ============================================================================
// RUSTEM SUITE — Crate rustem-accounting
// Plan de cuentas, asientos, libro mayor, balance VEN-NIF.
// ============================================================================

//! # Crate `rustem-accounting`
//! Contabilidad en partida doble bimoneda (VES/USD):
//! - Plan de cuentas VEN-NIF (jerárquico).
//! - Motor de asientos con invariante ∑débitos = ∑créditos.
//! - Generación automática de asientos desde ventas e inventario.

pub mod engine;
pub mod models;

pub use engine::EntryEngine;
pub use models::{Account, EntryLine, JournalEntry};

#[cfg(test)]
mod tests {
    #[test]
    fn accounting_smoke_test() {
        let _ = ();
    }
}
