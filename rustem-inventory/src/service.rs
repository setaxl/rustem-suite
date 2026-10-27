// ============================================================================
// RUSTEM SUITE — rustem-inventory::service
// Servicio de inventario: valoración FIFO/AVCO, kardex.
// ============================================================================

use crate::models::{MovementType, Product, StockMovement, Warehouse};

/// Servicio de operaciones de inventario.
pub struct InventoryService;

impl InventoryService {
    /// Crea una nueva instancia del servicio.
    pub fn new() -> Self {
        Self
    }

    /// Calcula el stock neto de un producto en un almacén a partir de sus
    /// movimientos.
    pub fn calculate_stock(
        movements: &[StockMovement],
        product_id: &str,
        warehouse_id: &str,
    ) -> i64 {
        movements
            .iter()
            .filter(|m| m.product_id == product_id && m.warehouse_id == warehouse_id)
            .fold(0i64, |acc, m| match m.movement_type {
                MovementType::Entrada => acc + m.quantity,
                MovementType::Salida => acc - m.quantity,
                MovementType::Transferencia => acc - m.quantity,
                MovementType::Ajuste => acc + m.quantity,
            })
    }

    /// Verifica si una salida de stock dejaría el inventario negativo.
    pub fn would_go_negative(
        movements: &[StockMovement],
        product_id: &str,
        warehouse_id: &str,
        outgoing_quantity: i64,
    ) -> bool {
        let current = Self::calculate_stock(movements, product_id, warehouse_id);
        current - outgoing_quantity < 0
    }
}

impl Default for InventoryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_stock_basic() {
        let movements = vec![
            StockMovement {
                id: "m1".to_string(),
                product_id: "p1".to_string(),
                warehouse_id: "w1".to_string(),
                movement_type: MovementType::Entrada,
                quantity: 100,
                unit_cost_cents: 1000,
                reference: Some("FACT-001".to_string()),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            StockMovement {
                id: "m2".to_string(),
                product_id: "p1".to_string(),
                warehouse_id: "w1".to_string(),
                movement_type: MovementType::Salida,
                quantity: 30,
                unit_cost_cents: 1000,
                reference: None,
                created_at: "2026-01-02T00:00:00Z".to_string(),
            },
        ];
        let stock = InventoryService::calculate_stock(&movements, "p1", "w1");
        assert_eq!(stock, 70);
    }

    #[test]
    fn would_go_negative() {
        let movements = vec![StockMovement {
            id: "m1".to_string(),
            product_id: "p1".to_string(),
            warehouse_id: "w1".to_string(),
            movement_type: MovementType::Entrada,
            quantity: 10,
            unit_cost_cents: 1000,
            reference: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }];
        assert!(!InventoryService::would_go_negative(&movements, "p1", "w1", 5));
        assert!(InventoryService::would_go_negative(&movements, "p1", "w1", 15));
    }
}
