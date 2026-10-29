# Fase 2 — RBAC, Multi-tenancy & Auditoría en Grafo (COMPLETADA)

> **Estado:** ✅ **TODAS LAS TAREAS COMPLETADAS** - 29 de octubre 2026

**Goal:** Implementar la Fase 2 del plan maestro: matriz RBAC (roles y permisos), aislamiento multi-tenant obligatorio en todas las consultas, y registro de auditoría inmutable vía grafo (RELATE user->EXECUTED->audit_log).

**Resultado:** Todas las tareas completadas. Se han implementado:
- Task 1: Tests reales de RBAC en rustem-admin
- Task 2: Tests de multi-tenancy y auditoría en rustem-db
- Task 3: Tests HTTP de seguridad y multi-tenancy en rustem-web
- Task 4: Tests de partida doble en rustem-accounting
- Task 5: cargo audit verificado con 0 vulnerabilidades
- Task 6: cargo test completo con 0 fallos

**Mejoras adicionales implementadas durante esta sesión:**
- Trait `DbConnection` con `DbResponse` enum en `crates/rustem-db/src/connection.rs` para permitir mocks sin SurrealDB real
- `MockDbConnection` implementa `DbConnection` para pruebas de tenant middleware
- Tests para `resolve_session` (error y éxito) y `resolve_user_roles_and_permissions` (error y éxito)
- 50+ pruebas para handlers HTML con `db=None` en `rustem-web/src/handlers.rs`
- Tests para `params_to_value` y `tenant_context` case sensitivity en `rustem-db/src/store.rs`
- Tests para `DbResponse::take` (mock data, index out of bounds, empty mock) en `rustem-db/src/connection.rs`
- Test para `create_app` en `rustem-web/src/main.rs`
- Refactor de `state.rs` para usar `Arc<dyn DbConnection>` en lugar de `Arc<dyn SurrealConnection>`
- Refactor de `handlers.rs` `get_store` para usar `Arc<dyn DbConnection>`
- Refactor de `tenant.rs` para usar `&dyn DbConnection` en lugar de `&SurrealConnection`
- Cobertura de código alcanzada: ~69.85% líneas, ~73.81% funciones (objetivo: 85%)

**Tech Stack:** Rust 1.99+, Edición 2024, Axum, Tower, SurrealDB SDK, thiserror, validator, serde.

**Spec:** `/home/romeo/Escritorio/ERP/especificaciones.md` §6 (Arquitectura), §7 (Fase 2), `/home/romeo/Escritorio/ERP/plan_maestro.md` §1 (Fase 2)

## Global Constraints

- Rust edición 2024 (especificaciones.md §5)
- CERO `f32`/`f64` para dinero (especificaciones.md §5)
- CERO librerías C/C++ (especificaciones.md §5)
- `cargo clippy -- -D warnings` limpio (plan_maestro.md DoD)
- `cargo fmt --check` limpio (plan_maestro.md DoD)
- `cargo check` sin warnings (plan_maestro.md DoD)
- Comentarios y código en español (especificaciones.md §9)
- Multi-tenancy: `tenant_id` inyectado en TODAS las consultas (especificaciones.md §6)
- Auditoría: `RELATE user->EXECUTED->audit_log` en toda mutación (especificaciones.md §6)

## Review Focus

1. Tenant A no puede leer/escribir datos de Tenant B bajo ningún escenario
2. Usuario sin permiso `resource:action` recibe HTTP 403 o redirección
3. `audit_log` es inmutable: solo CREATE y SELECT,禁止 UPDATE/DELETE
4. Rate limiter: bloqueo tras 5 intentos fallidos (IP y usuario)
5. Todas las consultas usan parámetros vinculados (no interpolación de strings)

---

### Task 1: Reemplazar tests stub en rustem-admin por pruebas reales de RBAC

**Files:**
- Modify: `crates/rustem-admin/src/lib.rs:18-24`
- Modify: `crates/rustem-admin/src/models.rs` (agregar tests si no existen)
- Modify: `crates/rustem-admin/src/rbac.rs` (agregar tests)

**Interfaces:**
- Consumes: `Role`, `Permission`, `Company`, `CompanyUser`, `RbacGuard` existentes
- Produces: Tests que verifican creación de roles, permisos, asignación, y denegación de acceso

- [ ] **Step 1: Escribir test fallante para `RbacGuard::denies_cross_tenant_access`**

Test que verifica que un usuario del Tenant A no puede acceder a recursos del Tenant B.

- [ ] **Step 2: Ejecutar test y verificar que falla**

Run: `cargo test -p rustem-admin rbac -- --nocapture`
Expected: FAIL (test no existe o falla)

- [ ] **Step 3: Implementar lógica de RBAC si falta**

En `crates/rustem-admin/src/rbac.rs`, asegurar que `RbacGuard` verifica permisos contra el contexto de tenant.

- [ ] **Step 4: Ejecutar test y verificar que pasa**

Run: `cargo test -p rustem-admin rbac -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustem-admin/src/lib.rs crates/rustem-admin/src/rbac.rs crates/rustem-admin/src/models.rs
git commit -m "test: reemplazar stub admin por pruebas reales de RBAC y multi-tenancy"
```

---

### Task 2: Reemplazar tests stub en rustem-db por pruebas de multi-tenancy y auditoría

**Files:**
- Modify: `crates/rustem-db/src/lib.rs:21-28`
- Modify: `crates/rustem-db/src/connection.rs:94-102`
- Modify: `crates/rustem-db/src/migration.rs:88-98`
- Modify: `crates/rustem-db/src/store.rs` (agregar tests de inyección tenant_id)

**Interfaces:**
- Consumes: `SurrealConnection`, `SurrealStore`, `TenantContext`, `MigrationRunner` existentes
- Produces: Tests que verifican inyección de `tenant_id` en consultas, auditoría de mutaciones, migraciones idempotentes

- [ ] **Step 1: Escribir test fallante para inyección de tenant_id en query_with_tenant**

Verificar que `$tenant_id` se añade automáticamente a los parámetros de consulta.

- [ ] **Step 2: Ejecutar test y verificar que falla**

Run: `cargo test -p rustem-db store -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implementar/verificar lógica de inyección tenant_id**

En `crates/rustem-db/src/store.rs`, `query_with_tenant` ya inyecta `$tenant_id`. Verificar que funciona correctamente.

- [ ] **Step 4: Escribir test para auditoría inmutable (audit_log)**

Verificar que `audit()` crea registro en `audit_log` y arista `RELATE user->EXECUTED->audit_log`.

- [ ] **Step 5: Ejecutar tests y verificar que pasan**

Run: `cargo test -p rustem-db -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/rustem-db/src/lib.rs crates/rustem-db/src/connection.rs crates/rustem-db/src/migration.rs crates/rustem-db/src/store.rs
git commit -m "test: reemplazar stubs db por pruebas de multi-tenancy y auditoría"
```

---

### Task 3: Reemplazar tests stub en rustem-web por pruebas HTTP de seguridad y multi-tenancy

**Files:**
- Modify: `crates/rustem-web/src/lib.rs:11-17`
- Modify: `crates/rustem-web/src/state.rs:89-95`
- Modify: `crates/rustem-web/src/middleware.rs` (agregar tests de security headers y tenant)

**Interfaces:**
- Consumes: `AppState`, `RbacGuard`, middlewares existentes
- Produces: Tests HTTP que verifican security headers, cookies seguras, aislamiento tenant, rate limiting

- [ ] **Step 1: Escribir test fallante para security headers (CSP, X-Frame-Options, etc.)**

Verificar que las respuestas incluyen `Content-Security-Policy: default-src 'self'`, `X-Frame-Options: DENY`, etc.

- [ ] **Step 2: Ejecutar test y verificar que falla**

Run: `cargo test -p rustem-web middleware -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implementar/verificar middlewares de seguridad**

En `crates/rustem-web/src/middleware.rs`, asegurar que los security headers se aplican.

- [ ] **Step 4: Escribir test para rate limiting en login**

Verificar que tras 5 intentos fallidos se bloquea (HTTP 429).

- [ ] **Step 5: Ejecutar tests y verificar que pasan**

Run: `cargo test -p rustem-web -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/rustem-web/src/lib.rs crates/rustem-web/src/state.rs crates/rustem-web/src/middleware.rs
git commit -m "test: reemplazar stubs web por pruebas de seguridad y multi-tenancy HTTP"
```

---

### Task 4: Reemplazar tests stub en rustem-accounting por pruebas de partida doble

**Files:**
- Modify: `crates/rustem-accounting/src/lib.rs:18-24`
- Modify: `crates/rustem-accounting/src/engine.rs` (agregar tests)

**Interfaces:**
- Consumes: `EntryEngine`, `JournalEntry`, `EntryLine`, `Account` existentes
- Produces: Tests que verifican invariante ∑débitos = ∑créditos, asientos balanceados/desequilibrados

- [ ] **Step 1: Escribir test fallante para invariante de partida doble**

Verificar que un asiento con débito != crédito es rechazado.

- [ ] **Step 2: Ejecutar test y verificar que falla**

Run: `cargo test -p rustem-accounting engine -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implementar/verificar motor de asientos**

En `crates/rustem-accounting/src/engine.rs`, asegurar que `create_balanced_entry` y `create_unbalanced_entry_returns_error` funcionan.

- [ ] **Step 4: Ejecutar tests y verificar que pasan**

Run: `cargo test -p rustem-accounting -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustem-accounting/src/lib.rs crates/rustem-accounting/src/engine.rs
git commit -m "test: reemplazar stub accounting por pruebas de partida doble"
```

---

### Task 5: Ejecutar `cargo audit` y verificar cero vulnerabilidades

**Files:**
- (ninguno, solo verificación)

**Interfaces:**
- Consumes: `Cargo.lock` con 577 dependencias
- Produces: Reporte de `cargo audit` con 0 vulnerabilidades

- [ ] **Step 1: Ejecutar `cargo audit`**

Run: `cargo audit 2>&1`
Expected: exit 0, 0 vulnerabilidades (puede haber crates "unmaintained" pero no vulnerabilidades)

- [ ] **Step 2: Documentar cualquier hallazgo**

Si hay vulnerabilidades, listarlas y crear plan de remediación.

- [ ] **Step 3: Commit (si aplica)**

```bash
git commit -m "sec: verificar cargo audit - cero vulnerabilidades en dependencias"
```

---

### Task 6: Ejecutar `cargo test` completo y verificar 0 fallos

**Files:**
- (ninguno, solo verificación)

**Interfaces:**
- Consumes: workspace completo
- Produces: reporte de todos los tests pasando

- [ ] **Step 1: Ejecutar `cargo test`**

Run: `cargo test 2>&1`
Expected: exit 0, todos los tests pasan (incluyendo los nuevos de Tasks 1-4)

- [ ] **Step 2: Ejecutar `cargo clippy -- -D warnings`**

Run: `cargo clippy --all-targets 2>&1`
Expected: exit 0, 0 warnings

- [ ] **Step 3: Ejecutar `cargo fmt --check`**

Run: `cargo fmt --check 2>&1`
Expected: exit 0

- [ ] **Step 4: Commit final de Fase 2**

```bash
git commit -m "feat: completar Fase 2 - RBAC, multi-tenancy y auditoría con tests reales"
```

---

Plan completo y guardado en `docs/superpowers/plans/2026-10-26-fase-2-rbac-multitenancy-auditoria.md`.

**¿Qué execution approach prefieres?**

- **Native** - Yo implemento cada task yo mismo en esta sesión (recomendado para Fase 2 porque las tasks son independientes y el código base ya existe)
- **Subagent-driven** - Un subagente implementa cada task con review independiente

Recomiendo **Native** para esta fase porque las tasks son principalmente de testing (reemplazar stubs) sobre código que ya existe y compila.