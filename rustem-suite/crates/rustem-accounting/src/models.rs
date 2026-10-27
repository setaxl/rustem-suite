// ============================================================================
// RUSTEM SUITE — rustem-accounting::models
// Modelo de plan de cuentas y asientos contables.
// ============================================================================

use serde::{Deserialize, Serialize};

/// Tipo de cuenta contable (VEN-NIF).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    Activo,
    Pasivo,
    Patrimonio,
    Ingresos,
    Costos,
    Gastos,
    Orden,
}

/// Cuenta del plan de cuentas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub level: u8,
    pub parent_code: Option<String>,
    pub is_operational: bool,
}

impl Account {
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        account_type: AccountType,
        level: u8,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            account_type,
            level,
            parent_code: None,
            is_operational: true,
        }
    }
}

/// Línea de un asiento contable (débito o crédito).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryLine {
    pub account_code: String,
    pub amount_cents: i64, // positivo = débito, negativo = crédito
    pub description: Option<String>,
}

/// Asiento contable (partida doble).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub date: String, // UTC ISO 8601
    pub description: String,
    pub lines: Vec<EntryLine>,
    pub tenant_id: String,
}

impl JournalEntry {
    /// Verifica la invariante de partida doble: ∑débitos = ∑créditos.
    pub fn is_balanced(&self) -> bool {
        let sum: i64 = self.lines.iter().map(|l| l.amount_cents).sum();
        sum == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_creation() {
        let a = Account::new("110101", "Caja", AccountType::Activo, 1);
        assert_eq!(a.code, "110101");
        assert_eq!(a.name, "Caja");
    }

    #[test]
    fn journal_entry_balanced() {
        let entry = JournalEntry {
            id: "e1".to_string(),
            date: "2026-01-01T00:00:00Z".to_string(),
            description: "Asiento de prueba".to_string(),
            lines: vec![
                EntryLine {
                    account_code: "110101".to_string(),
                    amount_cents: 10000, // débito
                    description: Some("Caja".to_string()),
                },
                EntryLine {
                    account_code: "410101".to_string(),
                    amount_cents: -10000, // crédito
                    description: Some("Ventas".to_string()),
                },
            ],
            tenant_id: "c1".to_string(),
        };
        assert!(entry.is_balanced());
    }

    #[test]
    fn journal_entry_unbalanced() {
        let entry = JournalEntry {
            id: "e2".to_string(),
            date: "2026-01-01T00:00:00Z".to_string(),
            description: "Asiento desbalanceado".to_string(),
            lines: vec![
                EntryLine {
                    account_code: "110101".to_string(),
                    amount_cents: 10000,
                    description: None,
                },
                EntryLine {
                    account_code: "410101".to_string(),
                    amount_cents: -5000,
                    description: None,
                },
            ],
            tenant_id: "c1".to_string(),
        };
        assert!(!entry.is_balanced());
    }

    #[test]
    fn account_new_with_different_types() {
        let a = Account::new("110101", "Caja", AccountType::Activo, 1);
        assert_eq!(a.code, "110101");
        assert_eq!(a.name, "Caja");
        assert_eq!(a.account_type, AccountType::Activo);
        assert_eq!(a.level, 1);
        assert!(a.is_operational);

        let b = Account::new("210101", "Banco", AccountType::Pasivo, 2);
        assert_eq!(b.code, "210101");
        assert_eq!(b.name, "Banco");
        assert_eq!(b.account_type, AccountType::Pasivo);
        assert_eq!(b.level, 2);

        let c = Account::new("310101", "Capital", AccountType::Patrimonio, 1);
        assert_eq!(c.account_type, AccountType::Patrimonio);

        let d = Account::new("410101", "Ventas", AccountType::Ingresos, 1);
        assert_eq!(d.account_type, AccountType::Ingresos);

        let e = Account::new("510101", "Costo", AccountType::Costos, 1);
        assert_eq!(e.account_type, AccountType::Costos);

        let f = Account::new("610101", "Gasto", AccountType::Gastos, 1);
        assert_eq!(f.account_type, AccountType::Gastos);

        let g = Account::new("710101", "Orden", AccountType::Orden, 1);
        assert_eq!(g.account_type, AccountType::Orden);
    }

    #[test]
    fn account_with_parent_code() {
        let mut a = Account::new("110101", "Caja", AccountType::Activo, 2);
        a.parent_code = Some("1101".to_string());
        assert_eq!(a.parent_code, Some("1101".to_string()));
    }

    #[test]
    fn account_not_operational() {
        let mut a = Account::new("110101", "Caja", AccountType::Activo, 1);
        a.is_operational = false;
        assert!(!a.is_operational);
    }
}
