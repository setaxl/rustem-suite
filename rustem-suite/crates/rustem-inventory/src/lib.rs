// ============================================================================
// RUSTEM SUITE — Crate rustem-inventory
// Inventario, productos, almacenes, FIFO/AVCO, Kardex.
// ============================================================================

//! # Crate `rustem-inventory`
//! Gestión de inventario multi-almacén con valoración VEN-NIF:
//! - Almacenes y productos con alícuotas de IVA.
//! - Movimientos inmutables y kardex auditable.
//! - Control de lotes con fecha de vencimiento.
//! - Algoritmos de valoración FIFO y Promedio Ponderado (AVCO) conforme a NIC 2.
//! - Invariante: stock negativo prohibido a menos que esté autorizado.

pub mod models;
pub mod service;

pub use models::{
    FifoLot, FifoValuation, KardexEntry, MovementType, Product, StockLot, StockMovement, Warehouse,
};
pub use service::{InventoryError, InventoryResult, InventoryService};

#[cfg(test)]
mod tests {
    use crate::InventoryService;

    #[test]
    fn inventory_smoke_test() {
        let svc = InventoryService::new();
        let _ = svc;
        let _ = ();
    }
}
