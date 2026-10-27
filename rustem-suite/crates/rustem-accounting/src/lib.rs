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
    use super::{Account, EntryEngine, EntryLine, JournalEntry};
    use crate::models::AccountType;

    #[test]
    fn accounting_entry_engine_creates_balanced_entry() {
        // Invariante de partida doble: ∑débitos = ∑créditos
        let entry = EntryEngine::create_entry(
            "e1",
            "2026-01-01",
            "Venta al contado",
            vec![
                EntryEngine::debit("110101", 10000),
                EntryEngine::credit("410101", 10000),
            ],
            "c1",
        )
        .unwrap();
        assert!(entry.is_balanced());
        assert_eq!(entry.id, "e1");
    }

    #[test]
    fn accounting_entry_engine_rejects_unbalanced_entry() {
        let result = EntryEngine::create_entry(
            "e2",
            "2026-01-01",
            "Desbalanceado",
            vec![
                EntryEngine::debit("110101", 10000),
                EntryEngine::credit("410101", 5000),
            ],
            "c1",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no balanceado"));
    }

    #[test]
    fn accounting_account_creation_uses_ven_nif_types() {
        let cuenta = Account::new("110101", "Caja", AccountType::Activo, 1);
        assert_eq!(cuenta.code, "110101");
        assert_eq!(cuenta.name, "Caja");
        assert!(cuenta.is_operational);
    }

    #[test]
    fn accounting_journal_entry_is_balanced_when_sum_is_zero() {
        let asiento = JournalEntry {
            id: "e3".to_string(),
            date: "2026-01-01".to_string(),
            description: "Asiento balanceado".to_string(),
            lines: vec![
                EntryLine {
                    account_code: "110101".to_string(),
                    amount_cents: 15000,
                    description: None,
                },
                EntryLine {
                    account_code: "410101".to_string(),
                    amount_cents: -10000,
                    description: None,
                },
                EntryLine {
                    account_code: "510101".to_string(),
                    amount_cents: -5000,
                    description: None,
                },
            ],
            tenant_id: "c1".to_string(),
        };
        assert!(asiento.is_balanced());
    }
}
