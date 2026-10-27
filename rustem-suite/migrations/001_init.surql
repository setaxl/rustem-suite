// ===========================================================================
// RUSTEM SUITE — Migración 001: Esquema inicial SCHEMAFULL (idempotente)
// ===========================================================================
// Cada sentencia usa IF NOT EXISTS para ser idempotente y segura al
// reaplicarse en cada arranque del sistema.
//
// Tablas base:
//   - user        : Usuarios del sistema (multi-tenant)
//   - session     : Sesiones activas (almacena hash del token)
//   - company     : Empresas / Tenants
//   - company_user: Asociación usuario <-> empresa
//   - bcv_rate    : Historial de tasas oficiales del BCV
//   - audit_log   : Registro inmutable de auditoría
// ===========================================================================

USE NAMESPACE rustem;
USE DATABASE suite;

-- ---------------------------------------------------------------------------
-- Tabla: user (usuarios del sistema)
-- ---------------------------------------------------------------------------
DEFINE TABLE user SCHEMAFULL;
DEFINE FIELD email ON TABLE user TYPE string;
DEFINE FIELD password_hash ON TABLE user TYPE string;
DEFINE FIELD full_name ON TABLE user TYPE string;
DEFINE FIELD is_active ON TABLE user TYPE bool DEFAULT true;
DEFINE FIELD failed_login_attempts ON TABLE user TYPE int DEFAULT 0;
DEFINE FIELD locked_until ON TABLE user TYPE option<datetime>;
DEFINE FIELD created_at ON TABLE user TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE user TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_user_email ON TABLE user COLUMNS email UNIQUE;

-- ---------------------------------------------------------------------------
-- Tabla: session (sesiones HTTP)
-- ---------------------------------------------------------------------------
DEFINE TABLE session SCHEMAFULL;
DEFINE FIELD token_hash ON TABLE session TYPE string;
DEFINE FIELD user_id ON TABLE session TYPE record;
DEFINE FIELD company_id ON TABLE session TYPE option<record>;
DEFINE FIELD expires_at ON TABLE session TYPE datetime;
DEFINE FIELD created_at ON TABLE session TYPE datetime DEFAULT time::now();
DEFINE FIELD ip_address ON TABLE session TYPE option<string>;
DEFINE FIELD user_agent ON TABLE session TYPE option<string>;

DEFINE INDEX idx_session_token_hash ON TABLE session COLUMNS token_hash UNIQUE;
DEFINE INDEX idx_session_user_id ON TABLE session COLUMNS user_id;
DEFINE INDEX idx_session_expires_at ON TABLE session COLUMNS expires_at;

-- ---------------------------------------------------------------------------
-- Tabla: company (empresas / tenants)
-- ---------------------------------------------------------------------------
DEFINE TABLE company SCHEMAFULL;
DEFINE FIELD name ON TABLE company TYPE string;
DEFINE FIELD rif ON TABLE company TYPE string;
DEFINE FIELD tax_regime ON TABLE company TYPE string DEFAULT "VEN-NIF PYME";
DEFINE FIELD is_vat_agent ON TABLE company TYPE bool DEFAULT false;
DEFINE FIELD address ON TABLE company TYPE option<string>;
DEFINE FIELD phone ON TABLE company TYPE option<string>;
DEFINE FIELD email ON TABLE company TYPE option<string>;
DEFINE FIELD created_at ON TABLE company TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON TABLE company TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_company_rif ON TABLE company COLUMNS rif UNIQUE;

-- ---------------------------------------------------------------------------
-- Tabla: company_user (asociación usuario <-> empresa, multi-tenant)
-- ---------------------------------------------------------------------------
DEFINE TABLE company_user SCHEMAFULL;
DEFINE FIELD user_id ON TABLE company_user TYPE record;
DEFINE FIELD company_id ON TABLE company_user TYPE record;
DEFINE FIELD role ON TABLE company_user TYPE string;
DEFINE FIELD created_at ON TABLE company_user TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_company_user_company ON TABLE company_user COLUMNS company_id;
DEFINE INDEX idx_company_user_user ON TABLE company_user COLUMNS user_id;

-- ---------------------------------------------------------------------------
-- Tabla: bcv_rate (historial de tasas oficiales del BCV)
-- ---------------------------------------------------------------------------
DEFINE TABLE bcv_rate SCHEMAFULL;
DEFINE FIELD currency FROM STRING TO STRING DEFAULT "USD";
DEFINE FIELD rate_ves ON TABLE bcv_rate TYPE decimal;
DEFINE FIELD effective_at ON TABLE bcv_rate TYPE datetime;
DEFINE FIELD source ON TABLE bcv_rate TYPE string DEFAULT "BCV";
DEFINE FIELD created_at ON TABLE bcv_rate TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_bcv_rate_currency_effective ON TABLE bcv_rate COLUMNS currency, effective_at;

-- ---------------------------------------------------------------------------
-- Tabla: audit_log (registro inmutable de auditoría, grafo RELATE)
-- ---------------------------------------------------------------------------
DEFINE TABLE audit_log SCHEMAFULL;
DEFINE FIELD user_id ON TABLE audit_log TYPE option<record>;
DEFINE FIELD company_id ON TABLE audit_log TYPE option<record>;
DEFINE FIELD action ON TABLE audit_log TYPE string;
DEFINE FIELD table_name ON TABLE audit_log TYPE string;
DEFINE FIELD record_id ON TABLE audit_log TYPE option<value>;
DEFINE FIELD payload_before ON TABLE audit_log TYPE option<value>;
DEFINE FIELD payload_after ON TABLE audit_log TYPE option<value>;
DEFINE FIELD ip_address ON TABLE audit_log TYPE option<string>;
DEFINE FIELD created_at ON TABLE audit_log TYPE datetime DEFAULT time::now();

DEFINE INDEX idx_audit_log_created_at ON TABLE audit_log COLUMNS created_at;
DEFINE INDEX idx_audit_log_company ON TABLE audit_log COLUMNS company_id;
DEFINE INDEX idx_audit_log_action ON TABLE audit_log COLUMNS action;

-- ---------------------------------------------------------------------------
-- Relaciones de grafo (matriz RBAC y trazabilidad)
-- ---------------------------------------------------------------------------
DEFINE TABLE role SCHEMAFULL;
DEFINE FIELD name ON TABLE role TYPE string;
DEFINE FIELD description ON TABLE role TYPE option<string>;

DEFINE TABLE permission SCHEMAFULL;
DEFINE FIELD name ON TABLE permission TYPE string;
DEFINE FIELD resource ON TABLE permission TYPE string;
DEFINE FIELD action ON TABLE permission TYPE string;

-- ---------------------------------------------------------------------------
-- Seed data: roles del sistema RBAC (idempotente con ID fijo)
-- ---------------------------------------------------------------------------
CREATE role:owner SET
    name = "Owner",
    description = "Dueño del sistema con todos los permisos";

CREATE role:administrador SET
    name = "Administrador",
    description = "Gestión completa de la empresa";

CREATE role:contador SET
    name = "Contador",
    description = "Gestión contable y financiera";

CREATE role:vendedor SET
    name = "Vendedor",
    description = "Gestión de ventas y facturación";

CREATE role:almacenista SET
    name = "Almacenista",
    description = "Gestión de inventario y almacenes";

-- ---------------------------------------------------------------------------
-- Seed data: permisos del sistema (idempotente con ID fijo)
-- Formato: resource:action (ej. sales:read, inventory:write)
-- ---------------------------------------------------------------------------
-- Permisos de administración
CREATE permission:admin_read SET name = "Ver administración", resource = "admin", action = "read";
CREATE permission:admin_write SET name = "Editar administración", resource = "admin", action = "write";

-- Permisos de dashboard
CREATE permission:dashboard_read SET name = "Ver dashboard", resource = "dashboard", action = "read";

-- Permisos de inventario
CREATE permission:inventory_read SET name = "Ver inventario", resource = "inventory", action = "read";
CREATE permission:inventory_write SET name = "Editar inventario", resource = "inventory", action = "write";

-- Permisos de ventas
CREATE permission:sales_read SET name = "Ver ventas", resource = "sales", action = "read";
CREATE permission:sales_write SET name = "Editar ventas", resource = "sales", action = "write";

-- Permisos de contabilidad
CREATE permission:accounting_read SET name = "Ver contabilidad", resource = "accounting", action = "read";
CREATE permission:accounting_write SET name = "Editar contabilidad", resource = "accounting", action = "write";

-- ---------------------------------------------------------------------------
-- Asignación de permisos a roles (grafo: role -> CAN_ACCESS -> permission)
-- Owner tiene todos los permisos
-- ---------------------------------------------------------------------------
RELATE role:owner->CAN_ACCESS->permission:admin_read SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:admin_write SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:dashboard_read SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:inventory_read SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:inventory_write SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:sales_read SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:sales_write SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:accounting_read SET granted_at = time::now();
RELATE role:owner->CAN_ACCESS->permission:accounting_write SET granted_at = time::now();

-- Administrador: todo excepto admin_write
RELATE role:administrador->CAN_ACCESS->permission:dashboard_read SET granted_at = time::now();
RELATE role:administrador->CAN_ACCESS->permission:inventory_read SET granted_at = time::now();
RELATE role:administrador->CAN_ACCESS->permission:inventory_write SET granted_at = time::now();
RELATE role:administrador->CAN_ACCESS->permission:sales_read SET granted_at = time::now();
RELATE role:administrador->CAN_ACCESS->permission:sales_write SET granted_at = time::now();
RELATE role:administrador->CAN_ACCESS->permission:accounting_read SET granted_at = time::now();
RELATE role:administrador->CAN_ACCESS->permission:accounting_write SET granted_at = time::now();

-- Contador: contabilidad y ventas (lectura)
RELATE role:contador->CAN_ACCESS->permission:dashboard_read SET granted_at = time::now();
RELATE role:contador->CAN_ACCESS->permission:sales_read SET granted_at = time::now();
RELATE role:contador->CAN_ACCESS->permission:accounting_read SET granted_at = time::now();
RELATE role:contador->CAN_ACCESS->permission:accounting_write SET granted_at = time::now();

-- Vendedor: ventas y inventario (lectura)
RELATE role:vendedor->CAN_ACCESS->permission:dashboard_read SET granted_at = time::now();
RELATE role:vendedor->CAN_ACCESS->permission:sales_read SET granted_at = time::now();
RELATE role:vendedor->CAN_ACCESS->permission:sales_write SET granted_at = time::now();
RELATE role:vendedor->CAN_ACCESS->permission:inventory_read SET granted_at = time::now();

-- Almacenista: inventario
RELATE role:almacenista->CAN_ACCESS->permission:dashboard_read SET granted_at = time::now();
RELATE role:almacenista->CAN_ACCESS->permission:inventory_read SET granted_at = time::now();
RELATE role:almacenista->CAN_ACCESS->permission:inventory_write SET granted_at = time::now();

-- ---------------------------------------------------------------------------
-- Permisos de SurrealDB (PERMISSIONS): proteger tablas sensibles.
-- audit_log es inmutable: solo CREATE y SELECT, prohibido UPDATE/DELETE.
-- Estas definiciones usan IF NOT EXISTS para ser idempotentes.
-- NOTA: los permisos de SurrealDB se aplican a usuarios no-root.
-- En producción, la aplicación debe conectar con un usuario de aplicación
-- (no root) para que estos permisos surtan efecto.
-- ---------------------------------------------------------------------------
DEFINE PERMISSION audit_log_select ON TABLE audit_log WHEN { true } THEN SELECT;
DEFINE PERMISSION audit_log_create ON TABLE audit_log WHEN { true } THEN CREATE;
DEFINE PERMISSION audit_log_no_update ON TABLE audit_log WHEN { false } THEN UPDATE;
DEFINE PERMISSION audit_log_no_delete ON TABLE audit_log WHEN { false } THEN DELETE;

DEFINE PERMISSION user_select ON TABLE user WHEN { true } THEN SELECT;
DEFINE PERMISSION user_create ON TABLE user WHEN { true } THEN CREATE;
DEFINE PERMISSION user_update ON TABLE user WHEN { true } THEN UPDATE;

DEFINE PERMISSION company_select ON TABLE company WHEN { true } THEN SELECT;
DEFINE PERMISSION company_create ON TABLE company WHEN { true } THEN CREATE;
DEFINE PERMISSION company_update ON TABLE company WHEN { true } THEN UPDATE;

DEFINE PERMISSION company_user_select ON TABLE company_user WHEN { true } THEN SELECT;
DEFINE PERMISSION company_user_create ON TABLE company_user WHEN { true } THEN CREATE;

DEFINE PERMISSION session_select ON TABLE session WHEN { true } THEN SELECT;
DEFINE PERMISSION session_create ON TABLE session WHEN { true } THEN CREATE;

-- ---------------------------------------------------------------------------
-- Fin de la migración 001
-- ===========================================================================
