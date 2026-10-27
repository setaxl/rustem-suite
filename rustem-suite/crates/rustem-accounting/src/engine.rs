// ============================================================================
// RUSTEM SUITE — rustem-accounting::engine
// Motor de asientos contables con invariante de partida doble.
// ============================================================================

use crate::models::{EntryLine, JournalEntry};

/// Motor para crear y validar asientos contables.
pub struct EntryEngine;

impl EntryEngine {
    /// Crea un asiento contable y verifica que esté balanceado.
    /// Devuelve error si ∑débitos != ∑créditos.
    pub fn create_entry(
        id: impl Into<String>,
        date: impl Into<String>,
        description: impl Into<String>,
        lines: Vec<EntryLine>,
        tenant_id: impl Into<String>,
    ) -> Result<JournalEntry, String> {
        let entry = JournalEntry {
            id: id.into(),
            date: date.into(),
            description: description.into(),
            lines,
            tenant_id: tenant_id.into(),
        };

        if !entry.is_balanced() {
            return Err(format!(
                "Asiento no balanceado: ∑débitos != ∑créditos (id: {})",
                entry.id
            ));
        }

        Ok(entry)
    }

    /// Crea una línea de débito.
    pub fn debit(account_code: impl Into<String>, amount_cents: i64) -> EntryLine {
        EntryLine {
            account_code: account_code.into(),
            amount_cents: amount_cents.abs(),
            description: None,
        }
    }

    /// Crea una línea de crédito.
    pub fn credit(account_code: impl Into<String>, amount_cents: i64) -> EntryLine {
        EntryLine {
            account_code: account_code.into(),
            amount_cents: -(amount_cents.abs()),
            description: None,
        }
    }
}

impl Default for EntryEngine {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_balanced_entry() {
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
    fn create_unbalanced_entry_returns_error() {
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
}
