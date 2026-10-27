// ============================================================================
// RUSTEM SUITE — rustem-inventory::service
// Servicio de inventario: movimientos ACID, stock negativo, FIFO/AVCO, kardex.
// ============================================================================

//! # Módulo `service`
//! Lógica de dominio de inventario VEN-NIF:
//! - Registro de movimientos de stock con validación de invariante (no stock
//!   negativo no autorizado).
//! - Algoritmo FIFO (Primero en Entrar, Primero en Salir) conforme a NIC 2.
//! - Algoritmo AVCO (Promedio Ponderado de Costos).
//! - Generación de kardex auditable por producto y almacén.
//!
//! Los métodos de valoración operan sobre colecciones en memoria, lo que
//! permite pruebas unitarias determinísticas sin necesidad de SurrealDB.

use crate::models::{FifoLot, FifoValuation, KardexEntry, MovementType, StockLot, StockMovement};

/// Errores del servicio de inventario.
#[derive(Debug, PartialEq, Eq)]
pub enum InventoryError {
    /// Stock insuficiente para la salida solicitada.
    InsufficientStock {
        product_id: String,
        warehouse_id: String,
        available: i64,
        requested: i64,
    },
    /// Producto no encontrado.
    ProductNotFound(String),
    /// Almacén no encontrado.
    WarehouseNotFound(String),
    /// Lote no encontrado.
    LotNotFound(String),
    /// Movimiento inválido (cantidad cero o negativa).
    InvalidQuantity(String),
}

/// Resultado tipificado del servicio de inventario.
pub type InventoryResult<T> = Result<T, InventoryError>;

/// Parámetros para una transferencia de stock entre almacenes.
#[derive(Debug, Clone)]
pub struct TransferParams {
    pub id: String,
    pub product_id: String,
    pub from_warehouse_id: String,
    pub to_warehouse_id: String,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub tenant_id: String,
}

/// Servicio de operaciones de inventario.
pub struct InventoryService;

impl InventoryService {
    /// Crea una nueva instancia del servicio.
    pub fn new() -> Self {
        Self
    }

    // ------------------------------------------------------------------
    // Movimientos de stock (F3-02)
    // ------------------------------------------------------------------

    /// Calcula el stock actual de un producto en un almacén a partir de sus
    /// movimientos históricos.
    pub fn calculate_stock(
        movements: &[StockMovement],
        product_id: &str,
        warehouse_id: &str,
    ) -> i64 {
        movements
            .iter()
            .filter(|m| m.product_id == product_id && m.warehouse_id == warehouse_id)
            .fold(0i64, |acc, m| {
                acc + m.quantity * m.movement_type.stock_sign()
            })
    }

    /// Verifica si una salida de stock dejaría el inventario negativo.
    /// Devuelve `Ok(())` si es permitida, `Err(InventoryError)` si viola el
    /// invariante de stock negativo.
    pub fn validate_no_negative_stock(
        movements: &[StockMovement],
        product_id: &str,
        warehouse_id: &str,
        outgoing_quantity: i64,
    ) -> InventoryResult<()> {
        let current = Self::calculate_stock(movements, product_id, warehouse_id);
        if current - outgoing_quantity < 0 {
            return Err(InventoryError::InsufficientStock {
                product_id: product_id.to_string(),
                warehouse_id: warehouse_id.to_string(),
                available: current,
                requested: outgoing_quantity,
            });
        }
        Ok(())
    }

    /// Crea un movimiento de entrada de stock.
    pub fn entry(
        id: impl Into<String>,
        product_id: impl Into<String>,
        warehouse_id: impl Into<String>,
        quantity: i64,
        unit_cost_cents: i64,
        tenant_id: String,
        reference: Option<String>,
    ) -> StockMovement {
        let mut m = StockMovement::new(
            id,
            product_id,
            warehouse_id,
            MovementType::Entrada,
            quantity,
            unit_cost_cents,
            tenant_id.as_str(),
        );
        m.reference = reference;
        m
    }

    /// Crea un movimiento de salida de stock.
    pub fn exit(
        id: impl Into<String>,
        product_id: impl Into<String>,
        warehouse_id: impl Into<String>,
        quantity: i64,
        unit_cost_cents: i64,
        tenant_id: String,
        reference: Option<String>,
    ) -> StockMovement {
        let mut m = StockMovement::new(
            id,
            product_id,
            warehouse_id,
            MovementType::Salida,
            quantity,
            unit_cost_cents,
            tenant_id.as_str(),
        );
        m.reference = reference;
        m
    }

    /// Crea un movimiento de ajuste de stock.
    pub fn adjustment(
        id: impl Into<String>,
        product_id: impl Into<String>,
        warehouse_id: impl Into<String>,
        quantity: i64, // positiva = aumento, negativa = disminución
        unit_cost_cents: i64,
        tenant_id: String,
        notes: Option<String>,
    ) -> StockMovement {
        let mut m = StockMovement::new(
            id,
            product_id,
            warehouse_id,
            MovementType::Ajuste,
            quantity.abs(),
            unit_cost_cents,
            tenant_id.as_str(),
        );
        m.notes = notes;
        m
    }

    /// Crea un movimiento de transferencia entre almacenes.
    pub fn transfer(params: TransferParams) -> (StockMovement, StockMovement) {
        let salida_id = format!("{}_salida", params.id);
        let entrada_id = format!("{}_entrada", params.id);
        let salida = StockMovement::new(
            salida_id,
            params.product_id.as_str(),
            params.from_warehouse_id,
            MovementType::Transferencia,
            params.quantity,
            params.unit_cost_cents,
            params.tenant_id.as_str(),
        );
        let entrada = StockMovement::new(
            entrada_id,
            params.product_id,
            params.to_warehouse_id,
            MovementType::Entrada,
            params.quantity,
            params.unit_cost_cents,
            params.tenant_id,
        );
        (salida, entrada)
    }

    // ------------------------------------------------------------------
    // Valoración FIFO (F3-03)
    // ------------------------------------------------------------------

    /// Valora una cantidad de stock usando el algoritmo FIFO.
    ///
    /// Los lotes deben estar ordenados por fecha de entrada (más antiguo
    /// primero). El algoritmo consume los lotes en orden hasta completar la
    /// cantidad solicitada.
    ///
    /// # Errores
    /// Devuelve `InventoryError::InsufficientStock` si los lotes no tienen
    /// suficiente cantidad disponible.
    pub fn calculate_fifo(
        lots: &[StockLot],
        quantity_to_consume: i64,
    ) -> InventoryResult<FifoValuation> {
        if quantity_to_consume <= 0 {
            return Err(InventoryError::InvalidQuantity(
                "La cantidad a consumir debe ser positiva".to_string(),
            ));
        }

        let mut remaining = quantity_to_consume;
        let mut total_cost = 0i64;
        let mut total_consumed = 0i64;
        let mut lots_consumed = Vec::new();

        // Filtrar lotes con cantidad disponible, ordenados por creación.
        let available_lots: Vec<&StockLot> = lots.iter().filter(|l| l.quantity > 0).collect();

        for lot in available_lots {
            if remaining <= 0 {
                break;
            }
            let consume = remaining.min(lot.quantity);
            let cost = consume * lot.unit_cost_cents;
            total_cost += cost;
            total_consumed += consume;
            remaining -= consume;

            lots_consumed.push(FifoLot {
                lot_id: lot.id.clone(),
                quantity_consumed: consume,
                unit_cost_cents: lot.unit_cost_cents,
                cost_cents: cost,
            });
        }

        if remaining > 0 {
            return Err(InventoryError::InsufficientStock {
                product_id: lots
                    .first()
                    .map(|l| l.product_id.clone())
                    .unwrap_or_default(),
                warehouse_id: lots
                    .first()
                    .map(|l| l.warehouse_id.clone())
                    .unwrap_or_default(),
                available: total_consumed,
                requested: quantity_to_consume,
            });
        }

        let average_unit_cost = if total_consumed > 0 {
            total_cost / total_consumed
        } else {
            0
        };

        Ok(FifoValuation {
            total_quantity: total_consumed,
            total_cost_cents: total_cost,
            average_unit_cost_cents: average_unit_cost,
            lots: lots_consumed,
        })
    }

    // ------------------------------------------------------------------
    // Valoración AVCO / Promedio Ponderado (F3-03)
    // ------------------------------------------------------------------

    /// Calcula el costo promedio ponderado de un conjunto de lotes.
    ///
    /// El AVCO considera todos los lotes disponibles y calcula un costo
    /// unitario promedio ponderado por cantidad.
    pub fn calculate_avco(lots: &[StockLot]) -> FifoValuation {
        let available_lots: Vec<&StockLot> = lots.iter().filter(|l| l.quantity > 0).collect();

        let total_quantity: i64 = available_lots.iter().map(|l| l.quantity).sum();
        let total_cost_cents: i64 = available_lots
            .iter()
            .map(|l| l.quantity * l.unit_cost_cents)
            .sum();

        let average_unit_cost_cents = if total_quantity > 0 {
            total_cost_cents / total_quantity
        } else {
            0
        };

        let lots: Vec<FifoLot> = available_lots
            .iter()
            .map(|l| FifoLot {
                lot_id: l.id.clone(),
                quantity_consumed: l.quantity,
                unit_cost_cents: l.unit_cost_cents,
                cost_cents: l.quantity * l.unit_cost_cents,
            })
            .collect();

        FifoValuation {
            total_quantity,
            total_cost_cents,
            average_unit_cost_cents,
            lots,
        }
    }

    /// Calcula el costo de un lote individual después de una entrada, usando
    /// AVCO para actualizar el costo promedio del lote existente.
    pub fn update_lot_cost_avco(
        existing_quantity: i64,
        existing_cost_cents: i64,
        incoming_quantity: i64,
        incoming_cost_cents: i64,
    ) -> i64 {
        let total_qty = existing_quantity + incoming_quantity;
        if total_qty == 0 {
            return 0;
        }
        (existing_cost_cents + incoming_cost_cents) / total_qty
    }

    // ------------------------------------------------------------------
    // Kardex (F3-03)
    // ------------------------------------------------------------------

    /// Genera las entradas del kardex a partir de una lista de movimientos
    /// ordenados cronológicamente.
    ///
    /// Cada entrada refleja el estado del stock después del movimiento.
    pub fn build_kardex(
        movements: &[StockMovement],
        product_id: &str,
        warehouse_id: &str,
    ) -> Vec<KardexEntry> {
        let mut stock_after = 0i64;
        let mut cost_after_cents = 0i64;
        let mut entries = Vec::new();

        for m in movements
            .iter()
            .filter(|m| m.product_id == product_id && m.warehouse_id == warehouse_id)
        {
            let sign = m.movement_type.stock_sign();
            stock_after += m.quantity * sign;

            // Actualizar costo acumulado (AVCO simple para el kardex).
            match m.movement_type {
                MovementType::Entrada | MovementType::Ajuste => {
                    cost_after_cents += m.total_cost_cents;
                }
                MovementType::Salida | MovementType::Transferencia => {
                    if stock_after > 0 && cost_after_cents > 0 {
                        let avg = cost_after_cents / stock_after.max(1);
                        cost_after_cents = avg * stock_after;
                    }
                }
            }

            entries.push(KardexEntry {
                movement_id: m.id.clone(),
                product_id: m.product_id.clone(),
                warehouse_id: m.warehouse_id.clone(),
                movement_type: m.movement_type.clone(),
                quantity: m.quantity,
                unit_cost_cents: m.unit_cost_cents,
                stock_after,
                cost_after_cents,
                created_at: m.created_at.clone(),
            });
        }

        entries
    }

    /// Calcula el valor total del inventario de un producto en un almacén
    /// usando AVCO.
    pub fn inventory_value_avco(lots: &[StockLot]) -> (i64, i64, i64) {
        let val = Self::calculate_avco(lots);
        (
            val.total_quantity,
            val.total_cost_cents,
            val.average_unit_cost_cents,
        )
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
    use chrono::Utc;

    fn make_movement(
        id: &str,
        product_id: &str,
        warehouse_id: &str,
        mtype: MovementType,
        qty: i64,
        cost: i64,
    ) -> StockMovement {
        StockMovement::new(id, product_id, warehouse_id, mtype, qty, cost, "company-1")
    }

    fn make_lot(id: &str, product_id: &str, warehouse_id: &str, qty: i64, cost: i64) -> StockLot {
        StockLot {
            id: id.to_string(),
            lot_number: format!("LOTE-{}", id),
            product_id: product_id.to_string(),
            warehouse_id: warehouse_id.to_string(),
            expiry_date: None,
            quantity: qty,
            unit_cost_cents: cost,
            tenant_id: "company-1".to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn calculate_stock_basic() {
        let movements = vec![
            make_movement("m1", "p1", "w1", MovementType::Entrada, 100, 1000),
            make_movement("m2", "p1", "w1", MovementType::Salida, 30, 1000),
        ];
        let stock = InventoryService::calculate_stock(&movements, "p1", "w1");
        assert_eq!(stock, 70);
    }

    #[test]
    fn calculate_stock_different_product() {
        let movements = vec![
            make_movement("m1", "p1", "w1", MovementType::Entrada, 100, 1000),
            make_movement("m2", "p2", "w1", MovementType::Entrada, 50, 1000),
        ];
        let stock_p1 = InventoryService::calculate_stock(&movements, "p1", "w1");
        let stock_p2 = InventoryService::calculate_stock(&movements, "p2", "w1");
        assert_eq!(stock_p1, 100);
        assert_eq!(stock_p2, 50);
    }

    #[test]
    fn validate_no_negative_stock_ok() {
        let movements = vec![make_movement(
            "m1",
            "p1",
            "w1",
            MovementType::Entrada,
            100,
            1000,
        )];
        assert!(InventoryService::validate_no_negative_stock(&movements, "p1", "w1", 50).is_ok());
    }

    #[test]
    fn validate_no_negative_stock_insufficient() {
        let movements = vec![make_movement(
            "m1",
            "p1",
            "w1",
            MovementType::Entrada,
            10,
            1000,
        )];
        let err =
            InventoryService::validate_no_negative_stock(&movements, "p1", "w1", 50).unwrap_err();
        assert!(matches!(err, InventoryError::InsufficientStock { .. }));
    }

    #[test]
    fn entry_movement() {
        let m = InventoryService::entry(
            "e1",
            "p1",
            "w1",
            50,
            1000,
            "company-1".to_string(),
            Some("FACT-001".to_string()),
        );
        assert_eq!(m.movement_type, MovementType::Entrada);
        assert_eq!(m.quantity, 50);
        assert_eq!(m.reference.as_deref(), Some("FACT-001"));
    }

    #[test]
    fn exit_movement() {
        let m = InventoryService::exit("s1", "p1", "w1", 20, 1000, "company-1".to_string(), None);
        assert_eq!(m.movement_type, MovementType::Salida);
        assert_eq!(m.quantity, 20);
    }

    #[test]
    fn adjustment_movement() {
        let m = InventoryService::adjustment(
            "a1",
            "p1",
            "w1",
            -5,
            1000,
            "company-1".to_string(),
            Some("Conteo físico".to_string()),
        );
        assert_eq!(m.movement_type, MovementType::Ajuste);
        assert_eq!(m.quantity, 5);
        assert_eq!(m.notes.as_deref(), Some("Conteo físico"));
    }

    #[test]
    fn transfer_movement() {
        let params = TransferParams {
            id: "t1".to_string(),
            product_id: "p1".to_string(),
            from_warehouse_id: "w1".to_string(),
            to_warehouse_id: "w2".to_string(),
            quantity: 10,
            unit_cost_cents: 1000,
            tenant_id: "company-1".to_string(),
        };
        let (salida, entrada) = InventoryService::transfer(params);
        assert_eq!(salida.movement_type, MovementType::Transferencia);
        assert_eq!(salida.warehouse_id, "w1");
        assert_eq!(entrada.movement_type, MovementType::Entrada);
        assert_eq!(entrada.warehouse_id, "w2");
        assert_eq!(salida.quantity, 10);
        assert_eq!(entrada.quantity, 10);
    }

    #[test]
    fn fifo_basic_consumption() {
        let lots = vec![
            make_lot("l1", "p1", "w1", 10, 1000), // 10 unidades a 1000 c/u
            make_lot("l2", "p1", "w1", 20, 1500), // 20 unidades a 1500 c/u
        ];
        let val = InventoryService::calculate_fifo(&lots, 15).unwrap();
        assert_eq!(val.total_quantity, 15);
        // 10 * 1000 + 5 * 1500 = 10000 + 7500 = 17500
        assert_eq!(val.total_cost_cents, 17500);
        assert_eq!(val.average_unit_cost_cents, 1166); // 17500 / 15
        assert_eq!(val.lots.len(), 2);
        assert_eq!(val.lots[0].quantity_consumed, 10);
        assert_eq!(val.lots[1].quantity_consumed, 5);
    }

    #[test]
    fn fifo_insufficient_stock() {
        let lots = vec![make_lot("l1", "p1", "w1", 5, 1000)];
        let err = InventoryService::calculate_fifo(&lots, 10).unwrap_err();
        assert!(matches!(err, InventoryError::InsufficientStock { .. }));
    }

    #[test]
    fn fifo_exact_consumption() {
        let lots = vec![
            make_lot("l1", "p1", "w1", 10, 1000),
            make_lot("l2", "p1", "w1", 10, 2000),
        ];
        let val = InventoryService::calculate_fifo(&lots, 20).unwrap();
        assert_eq!(val.total_quantity, 20);
        assert_eq!(val.total_cost_cents, 30000); // 10*1000 + 10*2000
    }

    #[test]
    fn avco_basic() {
        let lots = vec![
            make_lot("l1", "p1", "w1", 10, 1000),
            make_lot("l2", "p1", "w1", 20, 2000),
        ];
        let val = InventoryService::calculate_avco(&lots);
        assert_eq!(val.total_quantity, 30);
        assert_eq!(val.total_cost_cents, 50000); // 10*1000 + 20*2000
        assert_eq!(val.average_unit_cost_cents, 1666); // 50000 / 30
    }

    #[test]
    fn avco_empty_lots() {
        let lots = vec![];
        let val = InventoryService::calculate_avco(&lots);
        assert_eq!(val.total_quantity, 0);
        assert_eq!(val.total_cost_cents, 0);
        assert_eq!(val.average_unit_cost_cents, 0);
    }

    #[test]
    fn build_kardex() {
        let movements = vec![
            make_movement("m1", "p1", "w1", MovementType::Entrada, 100, 1000),
            make_movement("m2", "p1", "w1", MovementType::Salida, 30, 1000),
            make_movement("m3", "p1", "w1", MovementType::Ajuste, 5, 1000),
        ];
        let entries = InventoryService::build_kardex(&movements, "p1", "w1");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].stock_after, 100);
        assert_eq!(entries[1].stock_after, 70);
        assert_eq!(entries[2].stock_after, 75);
    }

    #[test]
    fn inventory_value_avco() {
        let lots = vec![
            make_lot("l1", "p1", "w1", 10, 1000),
            make_lot("l2", "p1", "w1", 10, 2000),
        ];
        let (qty, cost, avg) = InventoryService::inventory_value_avco(&lots);
        assert_eq!(qty, 20);
        assert_eq!(cost, 30000);
        assert_eq!(avg, 1500);
    }

    #[test]
    fn update_lot_cost_avco() {
        // Lote existente: 10 unidades a 1000 c/u = 10000
        // Entrada: 10 unidades a 2000 c/u = 20000
        // Nuevo costo promedio: 30000 / 20 = 1500
        let new_avg = InventoryService::update_lot_cost_avco(10, 10000, 10, 20000);
        assert_eq!(new_avg, 1500);
    }

    #[test]
    fn invalid_quantity_fifo() {
        let lots = vec![make_lot("l1", "p1", "w1", 10, 1000)];
        let err = InventoryService::calculate_fifo(&lots, 0).unwrap_err();
        assert!(matches!(err, InventoryError::InvalidQuantity(_)));
    }
}
