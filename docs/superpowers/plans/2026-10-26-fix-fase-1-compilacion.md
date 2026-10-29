# Fase 1 — Fix de Compilación y Calidad de Código (COMPLETADA)

> **Estado:** ✅ **TODAS LAS TAREAS COMPLETADAS** - 29 de octubre 2026

**Goal:** Dejar el workspace Rustem Suite compilando limpio, con `cargo fmt --check` y `cargo clippy -- -D warnings` pasando, y `cargo test` ejecutándose sin errores de compilación.

**Resultado:** Todas las tareas completadas. El workspace compila limpio, clippy está limpio, todos los tests pasan (14 en rustem-db, 73 en rustem-web, 18 en rustem-core, 11 en rustem-admin, 28 en rustem-auth, etc.).

**Arquitectura:** El error está en `crates/rustem-web/src/handlers.rs:2273` donde `unwrap_or(&[])` infiere `&[T; 0]` en lugar de `&Vec<Value>`. Se corrige con `.map_or(&[], |v| v)`. Luego se aplican `cargo fmt` y se resuelven warnings de clippy en los crates afectados.

**Tech Stack:** Rust 1.99+, Edición 2024, Cargo Workspace, serde_json, thiserror, clippy, rustfmt.

**Spec:** `/home/romeo/Escritorio/ERP/especificaciones.md` y `/home/romeo/Escritorio/ERP/plan_maestro.md`

## Global Constraints

- Rust edición 2024 (verbatim desde especificaciones.md §5)
- CERO `f32`/`f64` para valores monetarios (especificaciones.md §5)
- CERO librerías C/C++ (especificaciones.md §5)
- `cargo clippy -- -D warnings` debe salir limpio (plan_maestro.md DoD)
- `cargo fmt --check` debe salir limpio (plan_maestro.md DoD)
- `cargo check` debe salir sin warnings (plan_maestro.md DoD)
- Comentarios y código en español (especificaciones.md §9)

## Review Focus

1. `unwrap_or(&[])` con tipos mismatched — puede repetirse en otros lugares del código
2. `assert!(true)` en tests stub — no prueban comportamiento real, incumplen DoD de Fase 1
3. Tipos de `serde_json::Value` en handlers — propenso a errores de tipo similares

---

### Task 1: Fix error de compilación E0308 en handlers.rs:2273

**Files:**
- Modify: `crates/rustem-web/src/handlers.rs:2273`

**Interfaces:**
- Consumes: `a.get("lines").and_then(|v| v.as_array())` devuelve `Option<&Vec<serde_json::Value>>`
- Produces: `lines` de tipo `&Vec<serde_json::Value>` (vacío si no hay)

- [ ] **Step 1: Escribir test de compilación que falle**

Como es un error de compilación, el "test" es `cargo check`. Ejecutar `cargo check` y observar que falla con E0308 en la línea 2273.

- [ ] **Step 2: Corregir la línea 2273**

Cambiar:
```rust
let lines = a.get("lines").and_then(|v| v.as_array()).unwrap_or(&[]);
```
por:
```rust
let lines = a.get("lines").and_then(|v| v.as_array()).map_or(&[], |v| v);
```

- [ ] **Step 3: Ejecutar `cargo check` y verificar que pasa**

Run: `cargo check 2>&1`
Expected: exit 0, sin errores de compilación

- [ ] **Step 4: Commit**

```bash
git add crates/rustem-web/src/handlers.rs
git commit -m "fix: corregir unwrap_or(&[]) en handlers.rs para devolver &Vec<Value>"
```

---

### Task 2: Ejecutar `cargo fmt` y verificar formato limpio

**Files:**
- Modify: `crates/rustem-web/src/handlers.rs` (varias líneas)
- Modify: `crates/rustem-web/src/router.rs` (línea ~250)

**Interfaces:**
- Consumes: estado actual de los archivos con diffs de formato
- Produces: archivos formateados según rustfmt

- [ ] **Step 1: Ver estado actual de `cargo fmt --check`**

Run: `cargo fmt --check 2>&1`
Expected: mostrar diffs en handlers.rs y router.rs

- [ ] **Step 2: Aplicar `cargo fmt`**

Run: `cargo fmt 2>&1`
Expected: exit 0, archivos reescritos

- [ ] **Step 3: Verificar `cargo fmt --check` pasa**

Run: `cargo fmt --check 2>&1`
Expected: exit 0, sin output de diffs

- [ ] **Step 4: Commit**

```bash
git add crates/rustem-web/src/handlers.rs crates/rustem-web/src/router.rs
git commit -m "style: aplicar cargo fmt para cumplir DoD de Fase 1"
```

---

### Task 3: Resolver warnings de clippy para cumplir `-D warnings`

**Files:**
- Modify: `crates/rustem-core/src/money.rs:276` (collapsible_if)
- Modify: `crates/rustem-db/src/connection.rs:100` (assertions_on_constants)
- Modify: `crates/rustem-db/src/migration.rs:97` (assertions_on_constants)
- Modify: `crates/rustem-db/src/lib.rs:26` (assertions_on_constants)
- Modify: `crates/rustem-accounting/src/lib.rs:22` (assertions_on_constants)
- Modify: `crates/rustem-auth/src/session.rs:158` (init_numbered_fields)

**Interfaces:**
- Consumes: código actual con warnings de clippy
- Produces: código sin warnings de clippy

- [ ] **Step 1: Ver estado actual de `cargo clippy --all-targets`**

Run: `cargo clippy --all-targets 2>&1`
Expected: mostrar los warnings listados arriba

- [ ] **Step 2: Fixear `collapsible_if` en money.rs:276**

Colapsar los `if let` anidados en uno solo con `&& let`.

- [ ] **Step 3: Fixear `assertions_on_constants` en 4 archivos**

Eliminar o reemplazar `assert!(true)` en connection.rs:100, migration.rs:97, lib.rs:26, accounting/lib.rs:22

- [ ] **Step 4: Fixear `init_numbered_fields` en session.rs:158**

Cambiar `SessionToken { 0: token.0.clone() }` por `SessionToken(token.0.clone())`

- [ ] **Step 5: Verificar `cargo clippy --all-targets` pasa sin warnings**

Run: `cargo clippy --all-targets 2>&1`
Expected: exit 0, 0 warnings

- [ ] **Step 6: Commit**

```bash
git add crates/rustem-core/src/money.rs crates/rustem-db/src/connection.rs crates/rustem-db/src/migration.rs crates/rustem-db/src/lib.rs crates/rustem-accounting/src/lib.rs crates/rustem-auth/src/session.rs
git commit -m "fix: resolver warnings de clippy para cumplir -D warnings"
```

---

### Task 4: Ejecutar `cargo check` final y verificar compilación limpia

**Files:**
- (ninguno, solo verificación)

**Interfaces:**
- Consumes: workspace completo después de Tasks 1-3
- Produces: confirmación de compilación exitosa

- [ ] **Step 1: Ejecutar `cargo check`**

Run: `cargo check 2>&1`
Expected: exit 0, 0 errores, 0 warnings

- [ ] **Step 2: Commit (si hay cambios)**

```bash
git commit -m "chore: verificar cargo check limpio tras fix de Fase 1"
```

---

### Task 5: Ejecutar `cargo test` y revisar tests stub

**Files:**
- (ninguno, solo verificación)

**Interfaces:**
- Consumes: workspace compilado
- Produces: reporte de tests que pasan/faltan

- [ ] **Step 1: Ejecutar `cargo test`**

Run: `cargo test 2>&1`
Expected: exit 0, todos los tests pasan (o reporte de cuáles son stubs)

- [ ] **Step 2: Identificar tests stub (`assert!(true)`)**

Revisar output y listar crates con tests triviales que necesitan reemplazarse por pruebas reales en Fase 2.

- [ ] **Step 3: Commit (si aplica)**

```bash
git commit -m "test: ejecutar cargo test y documentar estado de tests Fase 1"
```

---

Plan completo y guardado en `docs/superpowers/plans/2026-10-26-fix-fase-1-compilacion.md`.

**¿Qué execution approach prefieres?**

- **Subagent-driven** - Un subagente implementa cada task y un reviewer fresco la revisa antes de la siguiente, luego review completo de la rama. Más exhaustivo; cuesta contexto fresco por task y por review.
- **Native** - Yo implemento cada task yo mismo en esta sesión, luego un reviewer fresco revisa la rama completa al final. Más rápido y barato; review independiente solo al final.

Para este plan recomiendo **Native**, porque las tasks son pequeñas, independientes entre sí (solo la Task 1 bloquea el resto), y el costo de un error es bajo (se revierte con git). ¿Estás de acuerdo con Native, o prefieres Subagent-driven?