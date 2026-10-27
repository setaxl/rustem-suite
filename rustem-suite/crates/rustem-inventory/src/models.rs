// ============================================================================
// RUSTEM SUITE — rustem-inventory::models
// Modelos de inventario: productos, almacenes, lotes, movimientos, kardex.
// ============================================================================

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Almacén de inventario (multi-sucursal, multi-tenant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub is_active: bool,
    pub tenant_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Warehouse {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            name: name.into(),
            address: None,
            phone: None,
            email: None,
            is_active: true,
            tenant_id: tenant_id.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Producto del catálogo con alícuota de IVA aplicable (VEN-NIF / SENIAT).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: String,
    pub category: Option<String>,
    pub vat_rate: String, // "16", "8", "0", "exento"
    pub min_stock: i64,
    pub is_active: bool,
    pub tenant_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Product {
    pub fn new(
        id: impl Into<String>,
        sku: impl Into<String>,
        name: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            sku: sku.into(),
            name: name.into(),
            description: None,
            unit: "und".to_string(),
            category: None,
            vat_rate: "16".to_string(),
            min_stock: 0,
            is_active: true,
            tenant_id: tenant_id.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Devuelve la alícuota de IVA como porcentaje entero.
    pub fn vat_percentage(&self) -> u8 {
        match self.vat_rate.as_str() {
            "16" => 16,
            "8" => 8,
            "0" | "exento" => 0,
            _ => 16,
        }
    }
}

/// Lote de stock con fecha de vencimiento (control de lote VEN-NIF).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockLot {
    pub id: String,
    pub lot_number: String,
    pub product_id: String,
    pub warehouse_id: String,
    pub expiry_date: Option<String>,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub tenant_id: String,
    pub created_at: String,
}

impl StockLot {
    pub fn new(
        id: impl Into<String>,
        lot_number: impl Into<String>,
        product_id: impl Into<String>,
        warehouse_id: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            lot_number: lot_number.into(),
            product_id: product_id.into(),
            warehouse_id: warehouse_id.into(),
            expiry_date: None,
            quantity: 0,
            unit_cost_cents: 0,
            tenant_id: tenant_id.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Tipo de movimiento de inventario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Entrada,
    Salida,
    Transferencia,
    Ajuste,
}

impl MovementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MovementType::Entrada => "ENTRADA",
            MovementType::Salida => "SALIDA",
            MovementType::Transferencia => "TRANSFERENCIA",
            MovementType::Ajuste => "AJUSTE",
        }
    }

    /// Signo del movimiento para el cálculo de stock: Entrada y Ajuste suman,
    /// Salida y Transferencia (origen) restan.
    pub fn stock_sign(&self) -> i64 {
        match self {
            MovementType::Entrada => 1,
            MovementType::Salida => -1,
            MovementType::Transferencia => -1,
            MovementType::Ajuste => 1,
        }
    }
}

impl FromStr for MovementType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ENTRADA" => Ok(MovementType::Entrada),
            "SALIDA" => Ok(MovementType::Salida),
            "TRANSFERENCIA" => Ok(MovementType::Transferencia),
            "AJUSTE" => Ok(MovementType::Ajuste),
            _ => Err(format!("Tipo de movimiento inválido: {s}")),
        }
    }
}

impl std::fmt::Display for MovementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Movimiento inmutable de stock (registro del kardex).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: String,
    pub product_id: String,
    pub warehouse_id: String,
    pub stock_lot_id: Option<String>,
    pub movement_type: MovementType,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub total_cost_cents: i64,
    pub reference: Option<String>, // factura, pedido, ajuste, etc.
    pub notes: Option<String>,
    pub tenant_id: String,
    pub created_at: String,
    pub created_by: Option<String>,
}

impl StockMovement {
    pub fn new(
        id: impl Into<String>,
        product_id: impl Into<String>,
        warehouse_id: impl Into<String>,
        movement_type: MovementType,
        quantity: i64,
        unit_cost_cents: i64,
        tenant_id: impl Into<String>,
    ) -> Self {
        let total_cost_cents = quantity.abs() * unit_cost_cents;
        Self {
            id: id.into(),
            product_id: product_id.into(),
            warehouse_id: warehouse_id.into(),
            stock_lot_id: None,
            movement_type,
            quantity: quantity.abs(),
            unit_cost_cents,
            total_cost_cents,
            reference: None,
            notes: None,
            tenant_id: tenant_id.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            created_by: None,
        }
    }
}

/// Entrada del kardex: estado de stock después de un movimiento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KardexEntry {
    pub movement_id: String,
    pub product_id: String,
    pub warehouse_id: String,
    pub movement_type: MovementType,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub stock_after: i64,
    pub cost_after_cents: i64,
    pub created_at: String,
}

/// Resultado de valoración FIFO: lotes consumidos en orden de entrada.
#[derive(Debug, Clone)]
pub struct FifoValuation {
    pub total_quantity: i64,
    pub total_cost_cents: i64,
    pub average_unit_cost_cents: i64,
    pub lots: Vec<FifoLot>,
}

/// Lote individual en una valoración FIFO.
#[derive(Debug, Clone)]
pub struct FifoLot {
    pub lot_id: String,
    pub quantity_consumed: i64,
    pub unit_cost_cents: i64,
    pub cost_cents: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warehouse_creation() {
        let w = Warehouse::new("w1", "Almacén Principal", "company-1");
        assert_eq!(w.name, "Almacén Principal");
        assert_eq!(w.tenant_id, "company-1");
        assert!(w.is_active);
    }

    #[test]
    fn product_creation() {
        let p = Product::new("p1", "SKU-001", "Producto Demo", "company-1");
        assert_eq!(p.sku, "SKU-001");
        assert_eq!(p.vat_percentage(), 16);
    }

    #[test]
    fn product_vat_rates() {
        let mut p = Product::new("p1", "SKU-001", "Prod", "c1");
        p.vat_rate = "8".to_string();
        assert_eq!(p.vat_percentage(), 8);

        p.vat_rate = "0".to_string();
        assert_eq!(p.vat_percentage(), 0);

        p.vat_rate = "exento".to_string();
        assert_eq!(p.vat_percentage(), 0);
    }

    #[test]
    fn movement_stock_sign() {
        assert_eq!(MovementType::Entrada.stock_sign(), 1);
        assert_eq!(MovementType::Salida.stock_sign(), -1);
        assert_eq!(MovementType::Transferencia.stock_sign(), -1);
        assert_eq!(MovementType::Ajuste.stock_sign(), 1);
    }

    #[test]
    fn movement_from_str() {
        assert_eq!(
            "ENTRADA".parse::<MovementType>().unwrap(),
            MovementType::Entrada
        );
        assert_eq!(
            "SALIDA".parse::<MovementType>().unwrap(),
            MovementType::Salida
        );
        assert!("INVALIDO".parse::<MovementType>().is_err());
    }

    #[test]
    fn stock_movement_total_cost() {
        let m = StockMovement::new(
            "m1",
            "p1",
            "w1",
            MovementType::Entrada,
            10,
            500, // 5.00 c/u
            "company-1",
        );
        assert_eq!(m.total_cost_cents, 5000); // 10 * 500
    }

    #[test]
    fn stock_lot_creation() {
        let l = StockLot::new("l1", "LOTE-2026-001", "p1", "w1", "company-1");
        assert_eq!(l.lot_number, "LOTE-2026-001");
        assert_eq!(l.quantity, 0);
    }
}
