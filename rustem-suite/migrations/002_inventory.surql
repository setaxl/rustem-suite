// ===========================================================================
// RUSTEM SUITE — Migración 002: Esquema de Inventario (idempotente)
// ===========================================================================
// Tablas de inventario:
//   - warehouse      : Almacenes multi-sucursal
//   - product        : Productos del catálogo (con alícuota IVA)
//   - stock_lot      : Lotes de producto con fecha de vencimiento
//   - stock_movement : Movimientos inmutables de stock (kardex)
//   - warehouse_stock: Vía grafo RELATE warehouse->HAS_STOCK->product
// ===========================================================================

USE NAMESPACE rustem;
USE DATABASE suite;

-- ---------------------------------------------------------------------------
-- Tabla: warehouse (almacenes)
-- ---------------------------------------------------------------------------
DEFINE TABLE warehouse SCHEMAFULL;
DEFINE FIELD name ON TABLE warehouse TYPE string;
DEFINE FIELD address ON TABLE warehouse TYPE option<string>;
DEFINE FIELD phone ON TABLE warehouse TYPE option<string>;
DEFINE FIELD email ON TABLE warehouse TYPE option<string>;
DEFINE FIELD is_active ON TABLE warehouse TYPE bool DEFAULT true;
DEFINE FIELD tenant_id ON TABLE warehouse TYPE string;
DEFINE FIELD created_at ON TABLE warehouse TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE warehouse TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_warehouse_tenant ON TABLE warehouse COLUMNS tenant_id;
DEFINE INDEX idx_warehouse_name ON TABLE warehouse COLUMNS name;

-- ---------------------------------------------------------------------------
-- Tabla: product (productos del catálogo)
-- ---------------------------------------------------------------------------
DEFINE TABLE product SCHEMAFULL;
DEFINE FIELD sku ON TABLE product TYPE string;
DEFINE FIELD name ON TABLE product TYPE string;
DEFINE FIELD description ON TABLE product TYPE option<string>;
DEFINE FIELD barcode ON TABLE product TYPE option<string>; -- código de barra
DEFINE FIELD unit ON TABLE product TYPE string DEFAULT "und";
DEFINE FIELD category ON TABLE product TYPE option<string>;
DEFINE FIELD vat_rate ON TABLE product TYPE string DEFAULT "16"; -- 16, 8, 0, exento
DEFINE FIELD min_stock ON TABLE product TYPE int DEFAULT 0;
DEFINE FIELD is_active ON TABLE product TYPE bool DEFAULT true;
DEFINE FIELD tenant_id ON TABLE product TYPE string;
DEFINE FIELD created_at ON TABLE product TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE product TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_product_tenant ON TABLE product COLUMNS tenant_id;
DEFINE INDEX idx_product_sku ON TABLE product COLUMNS sku UNIQUE;
DEFINE INDEX idx_product_category ON TABLE product COLUMNS category;
DEFINE INDEX idx_product_barcode ON TABLE product COLUMNS barcode;

-- ---------------------------------------------------------------------------
-- Tabla: stock_lot (lotes con fecha de vencimiento)
-- ---------------------------------------------------------------------------
DEFINE TABLE stock_lot SCHEMAFULL;
DEFINE FIELD lot_number ON TABLE stock_lot TYPE string;
DEFINE FIELD product_id ON TABLE stock_lot TYPE record;
DEFINE FIELD warehouse_id ON TABLE stock_lot TYPE record;
DEFINE FIELD expiry_date ON TABLE stock_lot TYPE option<datetime>;
DEFINE FIELD quantity ON TABLE stock_lot TYPE int DEFAULT 0;
DEFINE FIELD unit_cost_cents ON TABLE stock_lot TYPE int DEFAULT 0;
DEFINE FIELD tenant_id ON TABLE stock_lot TYPE string;
DEFINE FIELD created_at ON TABLE stock_lot TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_stock_lot_product ON TABLE stock_lot COLUMNS product_id;
DEFINE INDEX idx_stock_lot_warehouse ON TABLE stock_lot COLUMNS warehouse_id;
DEFINE INDEX idx_stock_lot_expiry ON TABLE stock_lot COLUMNS expiry_date;

-- ---------------------------------------------------------------------------
-- Tabla: stock_movement (movimientos inmutables del kardex)
-- ---------------------------------------------------------------------------
DEFINE TABLE stock_movement SCHEMAFULL;
DEFINE FIELD product_id ON TABLE stock_movement TYPE record;
DEFINE FIELD warehouse_id ON TABLE stock_movement TYPE record;
DEFINE FIELD stock_lot_id ON TABLE stock_movement TYPE option<record>;
DEFINE FIELD movement_type ON TABLE stock_movement TYPE string; -- ENTRADA, SALIDA, TRANSFERENCIA, AJUSTE
DEFINE FIELD quantity ON TABLE stock_movement TYPE int;
DEFINE FIELD unit_cost_cents ON TABLE stock_movement TYPE int;
DEFINE FIELD reference ON TABLE stock_movement TYPE option<string>; -- factura, pedido, etc.
DEFINE FIELD notes ON TABLE stock_movement TYPE option<string>;
DEFINE FIELD tenant_id ON TABLE stock_movement TYPE string;
DEFINE FIELD created_at ON TABLE stock_movement TYPE datetime DEFAULT time::now();
DEFINE FIELD created_by ON TABLE stock_movement TYPE option<string>;

DEFINE INDEX idx_stock_movement_product ON TABLE stock_movement COLUMNS product_id;
DEFINE INDEX idx_stock_movement_warehouse ON TABLE stock_movement COLUMNS warehouse_id;
DEFINE INDEX idx_stock_movement_type ON TABLE stock_movement COLUMNS movement_type;
DEFINE INDEX idx_stock_movement_created ON TABLE stock_movement COLUMNS created_at;

-- ---------------------------------------------------------------------------
-- Relación de grafo: warehouse->HAS_STOCK->product (trazabilidad)
-- Tabla de borde para aristas de grafo entre almacenes y productos.
-- ---------------------------------------------------------------------------
DEFINE TABLE warehouse_stock SCHEMAFULL;
DEFINE FIELD inbound ON TABLE warehouse_stock TYPE option<record>;
DEFINE FIELD outbound ON TABLE warehouse_stock TYPE option<record>;
DEFINE FIELD quantity ON TABLE warehouse_stock TYPE int DEFAULT 0;
DEFINE FIELD tenant_id ON TABLE warehouse_stock TYPE string;
DEFINE FIELD created_at ON TABLE warehouse_stock TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_warehouse_stock_tenant ON TABLE warehouse_stock COLUMNS tenant_id;

-- ---------------------------------------------------------------------------
-- Permisos de SurrealDB (PERMISSIONS): proteger tablas de inventario.
-- audit_log es inmutable: solo CREATE y SELECT, prohibido UPDATE/DELETE.
-- Estas definiciones usan IF NOT EXISTS para ser idempotentes.
-- ---------------------------------------------------------------------------
DEFINE PERMISSION warehouse_select ON TABLE warehouse WHEN { true } THEN SELECT;
DEFINE PERMISSION warehouse_create ON TABLE warehouse WHEN { true } THEN CREATE;
DEFINE PERMISSION warehouse_update ON TABLE warehouse WHEN { true } THEN UPDATE;

DEFINE PERMISSION product_select ON TABLE product WHEN { true } THEN SELECT;
DEFINE PERMISSION product_create ON TABLE product WHEN { true } THEN CREATE;
DEFINE PERMISSION product_update ON TABLE product WHEN { true } THEN UPDATE;

DEFINE PERMISSION stock_lot_select ON TABLE stock_lot WHEN { true } THEN SELECT;
DEFINE PERMISSION stock_lot_create ON TABLE stock_lot WHEN { true } THEN CREATE;
DEFINE PERMISSION stock_lot_update ON TABLE stock_lot WHEN { true } THEN UPDATE;

DEFINE PERMISSION stock_movement_select ON TABLE stock_movement WHEN { true } THEN SELECT;
DEFINE PERMISSION stock_movement_create ON TABLE stock_movement WHEN { true } THEN CREATE;

DEFINE PERMISSION warehouse_stock_select ON TABLE warehouse_stock WHEN { true } THEN SELECT;
DEFINE PERMISSION warehouse_stock_create ON TABLE warehouse_stock WHEN { true } THEN CREATE;

-- ---------------------------------------------------------------------------
-- Fin de la migración 002
-- ===========================================================================
