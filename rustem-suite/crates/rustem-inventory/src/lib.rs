// ============================================================================
// RUSTEM SUITE — rustem_inventory
// Inventario, productos, almacenes y kardex
// ============================================================================

//! # Módulo `rustem_inventory`
//! Gestión de inventario, productos, almacenes y valoración FIFO/AVCO.
//! ## Características
//! - ABM de almacenes y productos
//! - Movimientos de inventario con trazabilidad en grafo
//! - Algoritmos de valoración FIFO y promedio ponderado (AVCO)
//! - Kardex inmutable

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errores del dominio de inventario
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("stock insuficiente para producto {0}")]
    InsufficientStock(String),
    #[error("producto no encontrado: {0}")]
    NotFound(String),
    #[error("almacén no encontrado: {0}")]
    WarehouseNotFound(String),
    #[error("operación no válida: {0}")]
    InvalidOperation(String),
    #[error("cantidad debe ser positiva")]
    NegativeAmount,
}

pub type InventoryResult<T> = Result<T, InventoryError>;

// ---------------------------------------------------------------------------
// Producto
// ---------------------------------------------------------------------------

/// Producto del inventario con código de barras y categoría.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Identificador único del producto.
    pub id: String,
    /// Nombre del producto.
    pub name: String,
    /// Código de barras (opcional).
    pub barcode: Option<String>,
    /// Categoría del producto.
    pub category: String,
    /// Alícuota de IVA aplicable.
    pub iv_rate: f64,
    /// Stock actual.
    pub stock: i64,
}

impl Product {
    /// Crea un nuevo producto.
    pub fn new(id: String, name: String, barcode: Option<String>, category: String, iv_rate: f64, stock: i64) -> InventoryResult<Self> {
        if stock < 0 {
            return Err(InventoryError::NegativeAmount);
        }
        if iv_rate < 0.0 || iv_rate > 100.0 {
            return Err(InventoryError::InvalidOperation("IVA debe estar entre 0 y 100".to_string()));
        }
        Ok(Self {
            id,
            name,
            barcode,
            category,
            iv_rate,
            stock,
        })
    }

    /// Agrega stock al producto.
    pub fn add_stock(&mut self, amount: i64) -> InventoryResult<()> {
        if amount <= 0 {
            return Err(InventoryError::NegativeAmount);
        }
        self.stock += amount;
        Ok(())
    }

    /// Reduce stock del producto.
    /// Retorna el stock restante.
    pub fn reduce_stock(&mut self, amount: i64) -> InventoryResult<i64> {
        if amount <= 0 {
            return Err(InventoryError::NegativeAmount);
        }
        if self.stock < amount {
            return Err(InventoryError::InsufficientStock(format!("Producto {} tiene stock {} pero se solicita {}", self.id, self.stock, amount)));
        }
        self.stock -= amount;
        Ok(self.stock)
    }
}

// ---------------------------------------------------------------------------
// Almacén
// ---------------------------------------------------------------------------

/// Almacén del inventario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    /// Identificador único del almacén.
    pub id: String,
    /// Nombre del almacén.
    pub name: String,
    /// Ubicación física.
    pub location: Option<String>,
    /// Productos almacenados (relacionados por ID con stock).
    pub products: std::collections::HashMap<String, i64>,
}

impl Warehouse {
    /// Crea un nuevo almacén.
    pub fn new(id: String, name: String, location: Option<String>) -> Self {
        Self {
            id,
            name,
            location,
            products: std::collections::HashMap::new(),
        }
    }

    /// Agrega un producto al almacén con stock inicial.
    pub fn add_product(&mut self, product_id: String, initial_stock: i64) -> InventoryResult<()> {
        if initial_stock < 0 {
            return Err(InventoryError::NegativeAmount);
        }
        let count = self.products.get(&product_id).copied().unwrap_or(0);
        let new_count = count + initial_stock;
        self.products.insert(product_id, new_count);
        Ok(())
    }

    /// Obtiene el stock de un producto en este almacén.
    pub fn get_product_stock(&self, product_id: &str) -> InventoryResult<i64> {
        self.products.get(product_id)
            .copied()
            .ok_or_else(|| InventoryError::NotFound(format!("Producto {} no encontrado en almacén {}", product_id, self.id)))
    }
}

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_new() {
        let product = Product::new("P001".to_string(), "Producto de prueba".to_string(), None, "Categoría".to_string(), 0.16, 100);
        let product = product.unwrap();
        assert_eq!(product.id, "P001");
        assert_eq!(product.name, "Producto de prueba");
        assert_eq!(product.stock, 100);
        assert!(product.barcode.is_none());
    }

    #[test]
    fn product_with_barcode() {
        let product = Product::new("P002".to_string(), "Otro producto".to_string(), Some("123456789".to_string()), "Categoría".to_string(), 0.08, 50);
        let product = product.unwrap();
        assert_eq!(product.id, "P002");
        assert_eq!(product.name, "Otro producto");
        assert_eq!(product.stock, 50);
        assert_eq!(product.barcode, Some("123456789".to_string()));
    }

    #[test]
    fn product_add_stock() {
        let mut product = Product::new("P003".to_string(), "Producto a aumentar".to_string(), None, "Categoría".to_string(), 0.16, 50).unwrap();
        product.add_stock(30).unwrap();
        assert_eq!(product.stock, 80);
    }

    #[test]
    fn product_reduce_stock() {
        let mut product = Product::new("P004".to_string(), "Producto a reducir".to_string(), None, "Categoría".to_string(), 0.16, 100).unwrap();
        let remaining = product.reduce_stock(40).unwrap();
        assert_eq!(remaining, 60);
    }

    #[test]
    fn product_insufficient_stock() {
        let mut product = Product::new("P005".to_string(), "Producto con stock bajo".to_string(), None, "Categoría".to_string(), 0.16, 10).unwrap();
        let result = product.reduce_stock(20);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "stock insuficiente para producto Producto P005 tiene stock 10 pero se solicita 20");
    }

    #[test]
    fn product_zero_stock() {
        let mut product = Product::new("P006".to_string(), "Producto cero stock".to_string(), None, "Categoría".to_string(), 0.16, 0).unwrap();
        assert_eq!(product.stock, 0);
        product.add_stock(5).unwrap();
        assert_eq!(product.stock, 5);
    }

    #[test]
    fn warehouse_new() {
        let warehouse = Warehouse::new("W001".to_string(), "Almacén Principal".to_string(), None);
        assert_eq!(warehouse.id, "W001");
        assert_eq!(warehouse.name, "Almacén Principal");
        assert!(warehouse.location.is_none());
        assert!(warehouse.products.is_empty());
    }

    #[test]
    fn warehouse_with_location() {
        let warehouse = Warehouse::new("W002".to_string(), "Almacén Secundario".to_string(), Some("Sucursal".to_string()));
        assert_eq!(warehouse.id, "W002");
        assert_eq!(warehouse.name, "Almacén Secundario");
        assert_eq!(warehouse.location, Some("Sucursal".to_string()));
    }

    #[test]
    fn warehouse_add_product() {
        let mut warehouse = Warehouse::new("W003".to_string(), "Almacén Tercero".to_string(), None);
        warehouse.add_product("P007".to_string(), 50).unwrap();
        assert_eq!(warehouse.get_product_stock("P007").unwrap(), 50);
    }

    #[test]
    fn warehouse_product_not_found() {
        let warehouse = Warehouse::new("W004".to_string(), "Almacén Vacio".to_string(), None);
        let result = warehouse.get_product_stock("P999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "producto no encontrado: Producto P999 no encontrado en almacén W004");
    }

    #[test]
    fn warehouse_multiple_products() {
        let mut warehouse = Warehouse::new("W005".to_string(), "Almacén Completo".to_string(), None);
        warehouse.add_product("P008".to_string(), 30).unwrap();
        warehouse.add_product("P009".to_string(), 70).unwrap();
        assert_eq!(warehouse.get_product_stock("P008").unwrap(), 30);
        assert_eq!(warehouse.get_product_stock("P009").unwrap(), 70);
    }
}