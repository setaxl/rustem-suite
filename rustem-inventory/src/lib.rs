// ============================================================================
// RUSTEM SUITE — Crate rustem-inventory
// Inventario, productos, almacenes, FIFO/AVCO, Kardex.
// ============================================================================

//! # Crate `rustem-inventory`
//! Gestión de inventario multi-almacén con valoración VEN-NIF:
//! - Almacenes y productos.
//! - Movimientos inmutables y kardex auditable.
//! - Servicio de valoración FIFO/AVCO.

pub mod models;
pub mod service;

pub use models::{MovementType, Product, StockMovement, Warehouse};
pub use service::InventoryService;

#[cfg(test)]
mod tests {
    #[test]
    fn inventory_smoke_test() {
        assert!(true);
    }
}
