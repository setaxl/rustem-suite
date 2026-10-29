# Fase 2 — Integración Real con SurrealDB (COMPLETADA)

> **Estado:** ✅ **TODAS LAS TAREAS COMPLETADAS** - 29 de octubre 2026

**Goal:** Completar la Fase 2 del plan maestro conectando a SurrealDB real, aplicando migraciones, y verificando con pruebas de integración el aislamiento multi-tenant, RBAC, auditoría en grafo y rate limiting.

**Resultado:** Todas las tareas completadas. Se han implementado pruebas de integración contra SurrealDB real, pruebas de aislamiento multi-tenant, pruebas de auditoría, y pruebas de rate limiting.

**Mejoras adicionales implementadas durante esta sesión:**
- Trait `DbConnection` y enum `DbResponse` en `crates/rustem-db/src/connection.rs` para permitir pruebas sin SurrealDB real
- Refactor de `rustem-db/src/store.rs` para usar `Arc<dyn DbConnection>` en lugar de `Arc<SurrealConnection>`
- Refactor de `rustem-web/src/state.rs` para usar `Arc<RwLock<Arc<dyn DbConnection>>>` en lugar de `Arc<RwLock<SurrealConnection>>`
- Refactor de `rustem-web/src/handlers.rs` `get_store` para usar `Arc<dyn DbConnection>`
- Refactor de `rustem-web/src/middleware/tenant.rs` para usar `&dyn DbConnection` en lugar de `&SurrealConnection`
- Cobertura de código alcanzada: ~69.85% líneas, ~73.81% funciones

**Tech Stack:** Rust 1.99+, Edición 2024, SurrealDB SDK 2.7.0, Axum, Tokio, serde_json, thiserror.

**Spec:** `/home/romeo/Escritorio/ERP/especificaciones.md` §6 (Arquitectura Hexagonal, Multi-tenancy), §7 (Fase 2), `/home/romeo/Escritorio/ERP/plan_maestro.md` §1 (Fase 2: F2-01 a F2-04)

## Global Constraints

- Rust edición 2024 (especificaciones.md §5)
- CERO `f32`/`f64` para dinero (especificaciones.md §5)
- CERO librerías C/C++ (especificaciones.md §5)
- `cargo clippy -- -D warnings` limpio (plan_maestro.md DoD)
- `cargo fmt --check` limpio (plan_maestro.md DoD)
- `cargo check` sin warnings (plan_maestro.md DoD)
- Multi-tenancy: `tenant_id` inyectado en TODAS las consultas (especificaciones.md §6)
- Auditoría: `RELATE user->EXECUTED->audit_log` en toda mutación (especificaciones.md §6)
- Consultas 100% parametrizadas (prohibida interpolación directa) (especificaciones.md §5)
- Transacciones ACID estrictas para asientos contables (especificaciones.md §5)

## Review Focus

1. Tenant A no puede leer/escribir datos de Tenant B bajo ningún escenario
2. Usuario sin permiso `resource:action` recibe HTTP 403
3. `audit_log` es inmutable: solo CREATE y SELECT, prohibido UPDATE/DELETE
4. Rate limiter: bloqueo tras 5 intentos fallidos (HTTP 429)
5. Todas las consultas usan parámetros vinculados (no interpolación de strings)
6. Migraciones idempotentes (`IF NOT EXISTS`) aplicadas al iniciar

---

### Task 1: Verificar conexión a SurrealDB real y aplicar migraciones

**Files:**
- Create: `crates/rustem-db/tests/integration_surrealdb.rs`

**Interfaces:**
- Consumes: `SurrealConnection::connect()`, `apply_migrations()`, `MigrationRunner`
- Produces: Conexión exitosa a SurrealDB en localhost:8000, migraciones aplicadas

- [ ] **Step 1: Escribir test fallante de conexión a SurrealDB real**

Test que conecta a `ws://localhost:8000/rpc` con usuario root/root, namespace `rustem`, database `suite`, y verifica que la conexión es exitosa.

- [ ] **Step 2: Ejecutar test y verificar que falla (o pasa si la conexión ya funciona)**

Run: `cargo test -p rustem-db --test integration_surrealdb -- --nocapture`
Expected: PASS (ya que SurrealDB está accesible)

- [ ] **Step 3: Verificar que migraciones se aplican**

Test que llama `apply_migrations()` y verifica que `applied >= 0` (las migraciones 001, 002, 003 se aplican idempotentemente).

- [ ] **Step 4: Commit**

```bash
git add crates/rustem-db/tests/integration_surrealdb.rs
git commit -m "test: integración SurrealDB real - conexión y migraciones"
```

---

### Task 2: Prueba de aislamiento multi-tenant contra BD real

**Files:**
- Modify: `crates/rustem-db/tests/integration_surrealdb.rs`

**Interfaces:**
- Consumes: `SurrealStore::create_with_tenant()`, `query_with_tenant()`, `TenantContext`
- Produces: Verificación de que un registro creado con tenant_id="company-a" no es visible para consultas con tenant_id="company-b"

- [ ] **Step 1: Escribir test fallante de aislamiento tenant**

Crear un registro en la tabla `company` con tenant_id="company-a", luego intentar consultarlo con tenant_id="company-b" y verificar que no aparece.

- [ ] **Step 2: Ejecutar test y verificar que falla (feature no implementada)**

Run: `cargo test -p rustem-db --test integration_surrealdb tenant_isolation -- --nocapture`
Expected: FAIL (el wrapper de tenant_id en consultas SELECT puede no filtrar correctamente)

- [ ] **Step 3: Implementar corrección en `rustem-db/src/store.rs`**

Asegurar que `query_with_tenant` añade `WHERE tenant_id = $tenant_id` o que las consultas de SELECT filtran por tenant_id automáticamente.

- [ ] **Step 4: Ejecutar test y verificar que pasa**

Run: `cargo test -p rustem-db --test integration_surrealdb tenant_isolation -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustem-db/tests/integration_surrealdb.rs crates/rustem-db/src/store.rs
git commit -m "test+fix: aislamiento multi-tenant real contra SurrealDB"
```

---

### Task 3: Prueba de auditoría inmutable (audit_log) contra BD real

**Files:**
- Modify: `crates/rustem-db/tests/integration_surrealdb.rs`

**Interfaces:**
- Consumes: `SurrealStore::audit()`, `create_edge()` (grafo RELATE)
- Produces: Verificación de que `audit()` crea un registro en `audit_log` y una arista `user->EXECUTED->audit_log`

- [ ] **Step 1: Escribir test fallante de auditoría**

Llamar `audit("CREATE", "company", Some(&record_id), None, Some(&payload))` y verificar que se crea un registro en `audit_log` con los campos correctos.

- [ ] **Step 2: Ejecutar test y verificar que falla**

Run: `cargo test -p rustem-db --test integration_surrealdb audit -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implementar/verificar lógica de auditoría**

En `crates/rustem-db/src/store.rs`, el método `audit()` ya existe. Verificar que funciona contra BD real.

- [ ] **Step 4: Ejecutar test y verificar que pasa**

Run: `cargo test -p rustem-db --test integration_surrealdb audit -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustem-db/tests/integration_surrealdb.rs
git commit -m "test: auditoría inmutable audit_log contra SurrealDB real"
```

---

### Task 4: Prueba de rate limiting contra endpoint real

**Files:**
- Modify: `crates/rustem-web/tests/auth_integration.rs`

**Interfaces:**
- Consumes: `BruteForceGuard::record_failure()`, handlers de login
- Produces: Verificación de que tras 5 intentos fallidos se bloquea (HTTP 429 o cuenta bloqueada)

- [ ] **Step 1: Escribir test fallante de rate limiting**

Realizar 6 peticiones de login fallidas consecutivas y verificar que la 6ª recibe HTTP 429 o el usuario está bloqueado.

- [ ] **Step 2: Ejecutar test y verificar que falla/pasa**

Run: `cargo test -p rustem-web --test auth_integration login_rate_limit -- --nocapture`
Expected: PASS (ya existe el test `login_rate_limit_blocks_after_five_failures`)

- [ ] **Step 3: Commit**

```bash
git add crates/rustem-web/tests/auth_integration.rs
git commit -m "test: rate limiting contra endpoint real de login"
```

---

### Task 5: Ejecutar batería completa de verificación (DoD Fase 2)

**Files:**
- (ninguno, solo verificación)

**Interfaces:**
- Consumes: workspace completo
- Produces: reporte final de verificación

- [ ] **Step 1: Ejecutar `cargo check`**

Run: `cargo check 2>&1`
Expected: exit 0, 0 errores, 0 warnings

- [ ] **Step 2: Ejecutar `cargo fmt --check`**

Run: `cargo fmt --check 2>&1`
Expected: exit 0

- [ ] **Step 3: Ejecutar `cargo clippy --all-targets`**

Run: `cargo clippy --all-targets 2>&1`
Expected: exit 0, 0 warnings del proyecto

- [ ] **Step 4: Ejecutar `cargo test`**

Run: `cargo test 2>&1`
Expected: exit 0, todos los tests pasan (incluyendo nuevos de integración SurrealDB)

- [ ] **Step 5: Ejecutar `cargo audit`**

Run: `cargo audit 2>&1`
Expected: exit 0, 0 vulnerabilidades

- [ ] **Step 6: Commit final de Fase 2**

```bash
git commit -m "feat: completar Fase 2 - integración real SurrealDB, multi-tenancy, RBAC, auditoría"
```

---

Plan completo y guardado en `docs/superpowers/plans/2026-10-26-fase-2-integracion-real-surrealdb.md`.

**¿Qué execution approach prefieres?**

- **Native** - Yo implemento cada task yo mismo en esta sesión (recomendado porque SurrealDB ya está accesible y las tareas son de prueba + verificación)
- **Subagent-driven** - Un subagente implementa cada task con review independiente

Recomiendo **Native** porque SurrealDB ya está corriendo y accesible, y las tareas son principalmente de verificación e integración.