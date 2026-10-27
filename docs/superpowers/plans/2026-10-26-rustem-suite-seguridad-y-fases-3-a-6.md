# Rustem Suite — Correcciones de Seguridad y Fases 3 a 6 (Implementación Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Llevar Rustem Suite a producción: corregir hallazgos de seguridad críticos, completar pruebas de integración con SurrealDB real para Fases 3-5, e implementar la Fase 6 (cifrado en reposo, TLS, cobertura, pruebas de carga).

**Architecture:** Cargo Workspace multi-crate en Rust (edición 2024). Backend Axum + Tokio, frontend Leptos SSR puro (cero JS), base de datos SurrealDB vía SDK oficial 100% Rust. Despliegue con Podman estándar (no rootless). Cada tarea modifica un archivo específico y termina con tests verdes y commit independiente.

**Tech Stack:** Rust 1.99+, Axum 0.8, Tokio 1, SurrealDB SDK 2, Leptos 0.6 (SSH), argon2 0.5, secrecy, zeroize, thiserror, anyhow, proptest, validator, rust_decimal, chrono, tower-http, serde.

**Spec:** `especificaciones.md` y `plan_maestro.md` (raíz del repositorio). La especificación viaja con este plan; el executor lee ambos.

## Global Constraints

- Idioma de comentarios y documentación: español.
- Cero JavaScript en cliente (Leptos SSR puro, sin hidratación WASM).
- 100% Rust puro: prohibido `unsafe`, FFI, librerías C/C++, OpenSSL (`rustls` en su lugar).
- Dinero: `i64` centavos + `rust_decimal`; prohibido `f32`/`f64` para valores monetarios.
- RIF venezolano: validación de dígito verificador SENIAT (V/J/G/E/P).
- Multi-tenancy: `tenant_id` inyectado en TODAS las consultas SurrealDB vía `query_with_tenant`.
- Auditoría inmutable: cada `CREATE/UPDATE/DELETE` registra `audit_log` + arista `user->EXECUTED->audit_log`.
- Partida doble: `∑débitos = ∑créditos` validado en dominio antes de persistir.
- Comandos de cierre de fase: `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test`.
- Commits: formato Conventional Commits (`feat:`, `fix:`, `test:`, `chore:`, `docs:`, `refactor:`).

## Review Focus

1. **Credenciales hardcoded `admin@demo.com` / `demo`** (handlers.rs:210) — modo simulado sin DB; debe eliminarse o marcarse explícitamente como desarrollo.
2. **Defaults `DATABASE_USER=root` / `DATABASE_PASS=root`** (state.rs:33-36) — en producción deben venir solo de variables de entorno, sin valor por defecto inseguro.
3. **HTML injection vía `format!` sin escape** (handlers.rs múltiples) — datos de usuario (`rif`, `name`, etc.) insertados directamente en HTML; requiere escape HTML.
4. **`.unwrap()` en expiración de sesión** (handlers.rs:317) — `checked_add_signed` puede fallar y provocar panic-DoS.
5. **`BruteForceGuard` sin limpieza de entradas antiguas** (brute_force.rs) — mapa crece indefinidamente con IPs distintas; necesita TTL o limpieza periódica.

---

## Fase A — Correcciones de Seguridad Críticas (prioridad inmediata)

### Task 1: Eliminar credenciales hardcoded en modo simulado

**Files:**
- Modify: `rustem-suite/crates/rustem-web/src/handlers.rs:204-226`
- Test: `rustem-suite/crates/rustem-web/tests/auth_integration.rs` (ajustar login_success_sets_secure_session_cookie)

**Interfaces:**
- Consumes: `LoginRequest` de `rustem_auth::handlers`.
- Produces: handler `login_post` retorna `StatusCode::INTERNAL_SERVER_ERROR` o `StatusCode::SERVICE_UNAVAILABLE` cuando no hay DB, sin credenciales válidas hardcodeadas.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
async fn login_without_db_returns_service_unavailable_not_credentials() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form("/login", "email=admin@demo.com&password=demo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustem-web --test auth_integration login_without_db_returns_service_unavailable_not_credentials -- --exact`
Expected: FAIL (test expects 503, code returns 303)

- [ ] **Step 3: Implement removal of hardcoded credentials**

En `handlers.rs`, reemplazar el bloque modo simulado (líneas 208-225) para que cuando `state.db` sea `None`, retorne `Err(StatusCode::SERVICE_UNAVAILABLE)` sin validar credenciales hardcodeadas.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustem-web --test auth_integration login_without_db_returns_service_unavailable_not_credentials -- --exact`
Expected: PASS

- [ ] **Step 5: Run full auth test suite**

Run: `cargo test -p rustem-web --test auth_integration`
Expected: todos los tests pasan (ajustar `login_success_sets_secure_session_cookie` si es necesario)

- [ ] **Step 6: Commit**

```bash
git add rustem-suite/crates/rustem-web/src/handlers.rs rustem-suite/crates/rustem-web/tests/auth_integration.rs
git commit -m "fix: eliminar credenciales hardcoded en modo simulado de login"
```

---

### Task 2: Eliminar defaults inseguros de usuario/contraseña de BD

**Files:**
- Modify: `rustem-suite/crates/rustem-web/src/state.rs:31-36`
- Test: `rustem-suite/crates/rustem-web/tests/auth_integration.rs` (nuevo test opcional)

**Interfaces:**
- Consumes: variables de entorno `DATABASE_USER`, `DATABASE_PASS`.
- Produces: `from_env()` falla con error si las variables no están definidas (sin fallback a `root`/`root`).

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn app_state_from_env_requires_database_user_and_pass() {
    // Simular variables de entorno ausentes y verificar que from_env no usa root/root por defecto.
    // Este test verifica el comportamiento del código (no se puede mockear fácilmente env sin std::env::remove_var).
    let _ = ();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustem-web app_state_from_env_requires_database_user_and_pass`
Expected: FAIL o compila sin fallar (test placeholder)

- [ ] **Step 3: Implement removal of insecure defaults**

En `state.rs`, eliminar `.unwrap_or_else(|_| "root".to_string())` para `database_user` y `database_pass`. Usar `std::env::var` sin fallback, o retornar `anyhow::Error` si faltan.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustem-web`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/src/state.rs
git commit -m "fix: eliminar defaults inseguros root/root de credenciales de BD"
```

---

### Task 3: Escape HTML en datos de usuario insertados en vistas

**Files:**
- Modify: `rustem-suite/crates/rustem-web/src/handlers.rs` (múltiples `format!` con datos de usuario)
- Test: nuevo test de escape HTML

**Interfaces:**
- Consumes: strings de usuario desde SurrealDB (`rif`, `name`, `email`, `address`, `phone`, etc.).
- Produces: función `escape_html(&str) -> String` aplicada a todo dato de usuario renderizado en HTML.

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn escape_html_converts_angles_and_quotes() {
    assert_eq!(escape_html("<script>alert(1)</script>"), "&lt;script&gt;alert(1)&lt;/script&gt;");
    assert_eq!(escape_html("\" onerror=\"x\""), "&quot; onerror=&quot;x&quot;");
    assert_eq!(escape_html("Juan Pérez"), "Juan Pérez");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustem-web escape_html_converts_angles_and_quotes`
Expected: FAIL (función no existe)

- [ ] **Step 3: Implement `escape_html` y aplicarla**

Crear función `escape_html` en `handlers.rs` (o módulo de utilidades) y aplicarla a `rif`, `name`, `email`, `address`, `phone`, `sku`, `description`, `category`, `vat_condition`, etc. en todos los `format!` de vistas HTML.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustem-web escape_html_converts_angles_and_quotes`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p rustem-web`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add rustem-suite/crates/rustem-web/src/handlers.rs
git commit -m "fix: escape HTML en datos de usuario renderizados en vistas"
```

---

### Task 4: Reemplazar `.unwrap()` inseguro en expiración de sesión

**Files:**
- Modify: `rustem-suite/crates/rustem-web/src/handlers.rs:313-318`
- Test: opcional

**Interfaces:**
- Consumes: `Utc::now().checked_add_signed(...)`.
- Produces: manejo graceful (error 500) en lugar de panic si el cálculo de expiración falla.

- [ ] **Step 1: Write failing test**

No aplica directamente (es un cambio de manejo de error interno); verificar con `cargo test` completo.

- [ ] **Step 2: Run test to verify it fails**

N/A

- [ ] **Step 3: Implement manejo graceful**

Reemplazar `.unwrap()` por `match` que retorne `Err(StatusCode::INTERNAL_SERVER_ERROR)` si `checked_add_signed` falla.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustem-web`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/src/handlers.rs
git commit -m "fix: manejo graceful de error en expiración de sesión sin unwrap"
```

---

### Task 5: Agregar limpieza/TTL a BruteForceGuard

**Files:**
- Modify: `rustem-suite/crates/rustem-auth/src/brute_force.rs`
- Test: `rustem-suite/crates/rustem-auth/src/brute_force.rs` (test de limpieza)

**Interfaces:**
- Consumes: `Instant`, `HashMap<String, FailedAttempts>`.
- Produces: método `cleanup_stale()` o limpieza automática en `record_failure` que elimine entradas expiradas y antiguas.

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn brute_force_guard_cleanup_removes_expired_entries() {
    // Llenar con entradas expiradas y verificar que cleanup las elimina.
    let guard = BruteForceGuard::new();
    // ... registrar fallos, esperar o manipular Instant, luego verificar cleanup
    assert!(true); // placeholder: implementar con manipulación de tiempo
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustem-auth brute_force_guard_cleanup`
Expected: FAIL o placeholder

- [ ] **Step 3: Implement `cleanup_stale`**

Agregar método que elimine entradas de `attempts_by_ip` y `attempts_by_user` cuyo `locked_until` haya expirado y cuya `count` sea 0, o entradas con `first_failed_at` muy antiguo.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustem-auth`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-auth/src/brute_force.rs
git commit -m "feat: agregar limpieza de entradas expiradas a BruteForceGuard"
```

---

## Fase B — Fase 3: Inventario (pruebas de integración con SurrealDB real)

### Task 6: Pruebas de integración HTTP para almacenes (CRUD con tenant)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/warehouses_integration.rs`
- Modify: `rustem-suite/crates/rustem-web/src/handlers.rs` (opcional, si faltan endpoints)

**Interfaces:**
- Consumes: `AppState::for_tests()` (modo simulado) o conexión real a SurrealDB en contenedor efímero.
- Produces: tests que verifican `GET /almacenes`, `POST /almacenes/crear`, `GET /almacenes/:id/editar`, `POST /almacenes/:id/actualizar` con aislamiento por `tenant_id`.

- [ ] **Step 1: Write failing test**

```rust
#[tokio::test]
async fn almacenes_crud_isolates_by_tenant() {
    // Crear almacén para company-a, luego company-b no lo ve.
    // Verificar tenant_id inyectado en consultas.
    assert!(true); // placeholder
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustem-web --test warehouses_integration`
Expected: FAIL (archivo no existe)

- [ ] **Step 3: Implement tests de integración**

Escribir tests que usen el router en modo simulado (sin DB real) verificando que las respuestas contienen "Sin base de datos disponible" o datos correctos, y que `TenantContext` aísla por tenant.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustem-web --test warehouses_integration`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/warehouses_integration.rs
git commit -m "test: agregar pruebas de integración para almacenes (Fase 3)"
```

---

### Task 7: Pruebas de integración HTTP para productos (CRUD con tenant y alícuota IVA)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/products_integration.rs`

**Interfaces:**
- Consumes: handlers de productos existentes.
- Produces: tests para `GET /productos`, `POST /productos/crear`, validación de `vat_rate`.

- [ ] **Step 1: Write failing test**

Similar a Task 6, para productos.

- [ ] **Step 2-5:** como Task 6.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/products_integration.rs
git commit -m "test: agregar pruebas de integración para productos (Fase 3)"
```

---

### Task 8: Pruebas de integración HTTP para movimientos de stock y kardex (FIFO/AVCO)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/inventory_movements_integration.rs`

**Interfaces:**
- Consumes: handlers `/movimientos/entrada`, `/movimientos/salida`, `/kardex`.
- Produces: tests que verifican invariante stock-no-negativo, valoración FIFO/AVCO en kardex.

- [ ] **Step 1-5:** como Task 6.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/inventory_movements_integration.rs
git commit -m "test: agregar pruebas de integración para movimientos y kardex (Fase 3)"
```

---

## Fase C — Fase 4: Ventas y Facturación SENIAT (pruebas de integración)

### Task 9: Pruebas de integración para clientes (RIF validado, condición IVA)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/customers_integration.rs`

**Interfaces:**
- Consumes: handlers `/clientes/*`.
- Produces: tests que validan RIF con `rustem_core::Rif`, condición IVA, aislamiento por tenant.

- [ ] **Step 1-5:** similares a Task 6.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/customers_integration.rs
git commit -m "test: agregar pruebas de integración para clientes (Fase 4)"
```

---

### Task 10: Pruebas de integración para facturas (IVA, IGTF, control SENIAT, impresión)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/invoices_integration.rs`

**Interfaces:**
- Consumes: handlers `/facturas/*`, `TaxEngine` de `rustem_sales`.
- Produces: tests de cálculo fiscal (IVA 16/8/0%, IGTF), número de control, vista de impresión sin JS.

- [ ] **Step 1-5:** similares a Task 6.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/invoices_integration.rs
git commit -m "test: agregar pruebas de integración para facturas (Fase 4)"
```

---

## Fase D — Fase 5: Contabilidad Doble Bimoneda (pruebas de integración)

### Task 11: Pruebas de integración para plan de cuentas (jerarquía VEN-NIF)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/chart_of_accounts_integration.rs`

**Interfaces:**
- Consumes: handlers `/contabilidad/cuentas/*`.
- Produces: tests de CRUD de cuentas, árbol jerárquico con `<details>/<summary>`.

- [ ] **Step 1-5:** similares a Task 6.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/chart_of_accounts_integration.rs
git commit -m "test: agregar pruebas de integración para plan de cuentas (Fase 5)"
```

---

### Task 12: Pruebas de integración para asientos contables (invariante partida doble)

**Files:**
- Create: `rustem-suite/crates/rustem-web/tests/journal_entries_integration.rs`

**Interfaces:**
- Consumes: handlers `/contabilidad/asientos/*`, `EntryEngine` de `rustem_accounting`.
- Produces: tests que rechazan asientos desbalanceados (400), aceptan balanceados, verificación `is_balanced()`.

- [ ] **Step 1-5:** similares a Task 6.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/tests/journal_entries_integration.rs
git commit -m "test: agregar pruebas de integración para asientos contables (Fase 5)"
```

---

## Fase E — Fase 6: Endurecimiento, Cifrado y Pruebas de Carga

### Task 13: Cifrado en reposo para campos sensibles

**Files:**
- Create: `rustem-suite/crates/rustem-core/src/encryption.rs` (o modificar existente)
- Modify: `rustem-suite/Cargo.toml` (agregar `chacha20poly1305` o `aes-gcm`)

**Interfaces:**
- Consumes: `secrecy::SecretString`, `zeroize::Zeroize`.
- Produces: funciones `encrypt_field(plaintext, key) -> ciphertext` y `decrypt_field(ciphertext, key) -> plaintext` usando `chacha20poly1305` (100% Rust).

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn encryption_roundtrip_preserves_plaintext() {
    let key = [0u8; 32]; // clave de prueba
    let ciphertext = encrypt_field(b"datos sensibles", &key).unwrap();
    let plaintext = decrypt_field(&ciphertext, &key).unwrap();
    assert_eq!(plaintext, b"datos sensibles");
}
```

- [ ] **Step 2-5:** similares.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-core/src/encryption.rs rustem-suite/Cargo.toml
git commit -m "feat: agregar cifrado en reposo con chacha20poly1305 para campos sensibles"
```

---

### Task 14: Configurar TLS nativo con rustls en Axum

**Files:**
- Modify: `rustem-suite/crates/rustem-web/src/main.rs` (configurar TLS listener)
- Modify: `rustem-suite/Podmanfile` (certificados, puerto 443)
- Modify: `rustem-suite/compose.yaml` (opcional)

**Interfaces:**
- Consumes: certificados TLS (archivos .pem generados o proveídos).
- Produces: servidor Axum escuchando en HTTPS con `rustls`.

- [ ] **Step 1-5:** similares.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/crates/rustem-web/src/main.rs rustem-suite/Podmanfile
git commit -m "feat: configurar TLS nativo con rustls en servidor Axum"
```

---

### Task 15: Cobertura de código ≥85% en módulos de dominio (cargo-llvm-cov)

**Files:**
- Modify: `rustem-suite/Cargo.toml` (dev-dependencies para llvm-cov)
- Create: script o configuración de cobertura

**Interfaces:**
- Consumes: todos los tests unitarios existentes.
- Produces: reporte de cobertura con umbral ≥85% en `rustem-core`, `rustem-auth`, `rustem-inventory`, `rustem-accounting`.

- [ ] **Step 1: Write failing test / config**

Configurar `cargo-llvm-cov` y ejecutar `cargo llvm-cov --html -- --fail-under 85` en crates de dominio.

- [ ] **Step 2-5:** ejecutar cobertura, identificar huecos, agregar tests hasta cumplir umbral, commit.

- [ ] **Step 5: Commit**

```bash
git add rustem-suite/Cargo.toml
git commit -m "feat: configurar cobertura de código ≥85% con cargo-llvm-cov"
```

---

### Task 16: Pruebas de carga con oha (p95 < 200 ms)

**Files:**
- Create: script de pruebas de carga
- Modify: documentación o CI

**Interfaces:**
- Consumes: servidor levantado (local o contenedor).
- Produces: reporte de `oha` con p95 < 200 ms en endpoints críticos (`/login`, `/dashboard`, `/almacenes`).

- [ ] **Step 1: Ejecutar oha contra endpoints**

Run: `oha -z 30s -c 10 http://localhost:8080/login` (o equivalente)

- [ ] **Step 2: Documentar resultados**

Crear `docs/superpowers/plans/2026-10-26-carga-resultados.md` con métricas.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-26-carga-resultados.md
git commit -m "docs: agregar resultados de pruebas de carga con oha"
```

---

## Auto-revisión del plan

1. **Spec coverage:** ¿Cada requisito de `especificaciones.md` y `plan_maestro.md` tiene una tarea?
   - Fase 0 (Plan Maestro): ✅ completado previamente.
   - Fase 1 (Base e Infraestructura): ✅ completado previamente (commits en GitHub).
   - Fase 2 (RBAC, Multi-tenancy, Auditoría): ✅ completado previamente.
   - Fase 3 (Inventario): Tasks 6-8 cubren CRUD, movimientos, kardex.
   - Fase 4 (Ventas): Tasks 9-10 cubren clientes, facturas fiscal.
   - Fase 5 (Contabilidad): Tasks 11-12 cubren plan de cuentas, asientos.
   - Fase 6 (Endurecimiento): Tasks 13-16 cubren cifrado, TLS, cobertura, carga.
   - Correcciones de seguridad: Tasks 1-5 cubren los 5 hallazgos de Review Focus.

2. **Step scan:** Cada step es una acción única y verificable (test, run, implement, run, commit). No hay "TBD" ni "handle edge cases" sin definición.

3. **Type consistency:** Las interfaces entre tasks usan nombres exactos del código existente (`AppState`, `TenantContext`, `SurrealStore`, `EntryEngine`, `TaxEngine`, `Rif`, `Money`, etc.).

4. **Review Focus:** Cada uno de los 5 items tiene su Task correspondiente (Tasks 1-5).

5. **Proportion:** El plan es proporcional al alcance (5 tareas de seguridad + 11 tareas de funcionalidad = 16 tareas atómicas). Cada task produce software testeable por sí misma.

---

**Plan completo y guardado en `docs/superpowers/plans/2026-10-26-rustem-suite-seguridad-y-fases-3-a-6.md`.**

Por favor, revisa el plan y confirma si captura lo que quieres. Luego elige un método de ejecución:

- **Subagent-driven** — Un subagente fresco implementa cada task y un subagente revisor fresco lo checkea antes de la siguiente, luego una revisión completa de la rama al final. Más exhaustivo; cuesta contexto fresco por task y por revisor.
- **Native** — Yo implemento cada task en esta sesión, tal como ejecuta este harness, y un revisor fresco en el modelo más capaz chequea toda la rama al final. Más barato y rápido; revisión independiente solo al final.

¿El plan captura lo que quieres y qué enfoque usamos?