// ===========================================================================
// RUSTEM SUITE — Migración 003: Contabilidad Bicontable Doble (VEN-NIF)
// ===========================================================================
// Tablas de contabilidad:
//   - chart_of_accounts : Plan de cuentas jerárquico VEN-NIF
//   - journal_entries   : Asientos contables inmutables (partida doble)
// ===========================================================================

USE NAMESPACE rustem;
USE DATABASE suite;

-- ---------------------------------------------------------------------------
-- Tabla: chart_of_accounts (plan de cuentas VEN-NIF jerárquico)
-- ---------------------------------------------------------------------------
DEFINE TABLE chart_of_accounts SCHEMAFULL;
DEFINE FIELD code ON TABLE chart_of_accounts TYPE string;
DEFINE FIELD name ON TABLE chart_of_accounts TYPE string;
DEFINE FIELD account_type ON TABLE chart_of_accounts TYPE string; -- Activo, Pasivo, Patrimonio, Ingresos, Costos, Gastos, Orden
DEFINE FIELD level ON TABLE chart_of_accounts TYPE int;
DEFINE FIELD parent_code ON TABLE chart_of_accounts TYPE option<string>;
DEFINE FIELD is_operational ON TABLE chart_of_accounts TYPE bool DEFAULT true;
DEFINE FIELD tenant_id ON TABLE chart_of_accounts TYPE string;
DEFINE FIELD created_at ON TABLE chart_of_accounts TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE chart_of_accounts TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_chart_of_accounts_tenant ON TABLE chart_of_accounts COLUMNS tenant_id;
DEFINE INDEX idx_chart_of_accounts_code ON TABLE chart_of_accounts COLUMNS code UNIQUE;
DEFINE INDEX idx_chart_of_accounts_parent ON TABLE chart_of_accounts COLUMNS parent_code;
DEFINE INDEX idx_chart_of_accounts_type ON TABLE chart_of_accounts COLUMNS account_type;

-- ---------------------------------------------------------------------------
-- Tabla: journal_entries (asientos contables inmutables, partida doble)
-- ---------------------------------------------------------------------------
DEFINE TABLE journal_entries SCHEMAFULL;
DEFINE FIELD id ON TABLE journal_entries TYPE string;
DEFINE FIELD date ON TABLE journal_entries TYPE datetime;
DEFINE FIELD description ON TABLE journal_entries TYPE string;
DEFINE FIELD lines ON TABLE journal_entries TYPE array<object>; -- [{account_code, amount_cents, description}]
DEFINE FIELD tenant_id ON TABLE journal_entries TYPE string;
DEFINE FIELD created_at ON TABLE journal_entries TYPE datetime DEFAULT time::now();
DEFINE FIELD created_by ON TABLE journal_entries TYPE option<string>;

DEFINE INDEX idx_journal_entries_tenant ON TABLE journal_entries COLUMNS tenant_id;
DEFINE INDEX idx_journal_entries_date ON TABLE journal_entries COLUMNS date;
DEFINE INDEX idx_journal_entries_id ON TABLE journal_entries COLUMNS id UNIQUE;

-- ---------------------------------------------------------------------------
-- Seed data: Plan de cuentas VEN-NIF básico (idempotente con ID fijo)
-- Estructura jerárquica con códigos contables venezolanos
-- ---------------------------------------------------------------------------
-- Activo (1xxx)
CREATE chart_of_accounts:1101 SET
    code = "1101",
    name = "Caja y Bancos",
    account_type = "Activo",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:110101 SET
    code = "110101",
    name = "Caja",
    account_type = "Activo",
    level = 2,
    parent_code = "1101",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:110102 SET
    code = "110102",
    name = "Bancos",
    account_type = "Activo",
    level = 2,
    parent_code = "1101",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:1102 SET
    code = "1102",
    name = "Cuentas por Cobrar",
    account_type = "Activo",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:110201 SET
    code = "110201",
    name = "Clientes",
    account_type = "Activo",
    level = 2,
    parent_code = "1102",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:1103 SET
    code = "1103",
    name = "Inventario",
    account_type = "Activo",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:110301 SET
    code = "110301",
    name = "Mercancías",
    account_type = "Activo",
    level = 2,
    parent_code = "1103",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- Pasivo (2xxx)
CREATE chart_of_accounts:2101 SET
    code = "2101",
    name = "Cuentas por Pagar",
    account_type = "Pasivo",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:210101 SET
    code = "210101",
    name = "Proveedores",
    account_type = "Pasivo",
    level = 2,
    parent_code = "2101",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:2102 SET
    code = "2102",
    name = "IVA Crédito Fiscal",
    account_type = "Pasivo",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- Patrimonio (3xxx)
CREATE chart_of_accounts:3101 SET
    code = "3101",
    name = "Capital Social",
    account_type = "Patrimonio",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:3102 SET
    code = "3102",
    name = "Utilidades Retenidas",
    account_type = "Patrimonio",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- Ingresos (4xxx)
CREATE chart_of_accounts:4101 SET
    code = "4101",
    name = "Ventas",
    account_type = "Ingresos",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:410101 SET
    code = "410101",
    name = "Ventas de Mercancías",
    account_type = "Ingresos",
    level = 2,
    parent_code = "4101",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- Costos (5xxx)
CREATE chart_of_accounts:5101 SET
    code = "5101",
    name = "Costo de Ventas",
    account_type = "Costos",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

CREATE chart_of_accounts:510101 SET
    code = "510101",
    name = "Costo de Mercancías Vendidas",
    account_type = "Costos",
    level = 2,
    parent_code = "5101",
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- Gastos (6xxx)
CREATE chart_of_accounts:6101 SET
    code = "6101",
    name = "Gastos Administrativos",
    account_type = "Gastos",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- Cuentas de Orden (7xxx)
CREATE chart_of_accounts:7101 SET
    code = "7101",
    name = "Cuentas de Orden Deudoras",
    account_type = "Orden",
    level = 1,
    parent_code = NONE,
    is_operational = true,
    tenant_id = "system",
    created_at = time::now(),
    updated_at = time::now();

-- ---------------------------------------------------------------------------
-- Permisos de SurrealDB (PERMISSIONS): proteger tablas de contabilidad.
-- journal_entries es inmutable: solo CREATE y SELECT, prohibido UPDATE/DELETE.
-- chart_of_accounts permite CRUD completo para administración.
-- ---------------------------------------------------------------------------
DEFINE PERMISSION chart_of_accounts_select ON TABLE chart_of_accounts WHEN { true } THEN SELECT;
DEFINE PERMISSION chart_of_accounts_create ON TABLE chart_of_accounts WHEN { true } THEN CREATE;
DEFINE PERMISSION chart_of_accounts_update ON TABLE chart_of_accounts WHEN { true } THEN UPDATE;
DEFINE PERMISSION chart_of_accounts_delete ON TABLE chart_of_accounts WHEN { true } THEN DELETE;

DEFINE PERMISSION journal_entries_select ON TABLE journal_entries WHEN { true } THEN SELECT;
DEFINE PERMISSION journal_entries_create ON TABLE journal_entries WHEN { true } THEN CREATE;
DEFINE PERMISSION journal_entries_no_update ON TABLE journal_entries WHEN { false } THEN UPDATE;
DEFINE PERMISSION journal_entries_no_delete ON TABLE journal_entries WHEN { false } THEN DELETE;

-- ---------------------------------------------------------------------------
-- Fin de la migración 003
-- ===========================================================================
