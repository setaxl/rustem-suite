# PLAN MAESTRO DE DESARROLLO — RUSTEM SUITE (2026)

**Proyecto:** ERP Modular Full-Stack 100% Rust  
**Entorno de Trabajo:** Fedora Rawhide / Podman (Rootless Container Runtime)  
**Marco Normativo Fiscal & Contable:** VEN-NIF (GE / PYME) & SENIAT (Providencias SNAT, IGTF, Retenciones IVA/ISLR, Tasa BCV)  
**Restricción de Frontend:** CERO JavaScript / Leptos SSR Puro  
**Políticas Criptográficas y C++:** 100% Rust Pura (Cero librerías nativas C/C++)  

---

## 1. BACKLOG DETALLADO POR FASES (F1 – F6)

### FASE 1: BASE E INFRAESTRUCTURA CORE
*Objetivo:* Establecer el Cargo Workspace, la base de datos SurrealDB en Podman, migraciones idempotentes, la arquitectura hexagonal en Rust y el flujo completo de Autenticación con Cookies seguras y SSR.

* **F1-01 (M) — Configuración del Workspace y Toolchain en Fedora Rawhide:**
  * Crear estructura multi-crate (`rustem-core`, `rustem-auth`, `rustem-admin`, `rustem-db`, `rustem-inventory`, `rustem-sales`, `rustem-accounting`, `rustem-frontend`, `rustem-web`).
  * Configuración de `Cargo.toml` raíz con edición 2024 / Rust 1.99+ y perfiles de optimización.
  * *Entregable:* Workspace ejecutable con `cargo check` y `clippy` limpios.

* **F1-02 (L) — Capa de Persistencia `rustem-db` & SurrealDB 3.x en Podman:**
  * Scripting y ejecución de SurrealDB local usando Podman en Fedora.
  * Sistema de migraciones idempotentes `.surql` embebidas con `rust-embed`.
  * Esquema base `SCHEMAFULL` para usuarios, sesiones, empresas e historial de tasas BCV.
  * *Entregable:* `rustem-db` ejecutando consultas parametrizadas y aplicando esquema al iniciar.

* **F1-03 (L) — Crate `rustem-core`: Tipos de Dominio, VEN-NIF Money y Errores:**
  * Tipo `Money` con centavos en `i64`, código ISO 4217 (`VES`, `USD`) y operaciones exactas con `rust_decimal`.
  * Estructura RIF venezolano (`V-`, `J-`, `G-`, `E-`, `P-`) con algoritmo de cálculo/validación de dígito verificador.
  * *Entregable:* Suite de pruebas unitarias con `proptest` para invarianza de redondeo y validación RIF.

* **F1-04 (L) — Autenticación Segura (`rustem-auth`):**
  * Hashing de contraseñas con `argon2id` (configuración ajustada a estándares OWASP 2026).
  * Generación de tokens de sesión con `getrandom` (256-bit entropy) y almacenamiento de Hash SHA-256 en SurrealDB.
  * Cookies HTTP-Only, Secure, SameSite=Lax manejadas desde Axum.
  * *Entregable:* Handlers de login, logout, registro y recuperación con flujo PRG (Post-Redirect-Get).

* **F1-05 (M) — Vistas SSR con Leptos & Layout CSS Artesanal:**
  * Layout maestro HTML5 con Leptos SSR (CERO JS, sin `HydrationScripts`).
  * Estilos CSS artesanales con Design Tokens (CSS Variables) para modo claro/oscuro vía `:root` y `prefers-color-scheme`.
  * *Entregable:* Páginas de Login/Registro respondiendo HTML nativo estilizado.

---

### FASE 2: RBAC, MULTI-TENANCY & AUDITORÍA EN GRAFO
*Objetivo:* Garantizar el aislamiento estricto por empresa (*Tenant*) e implementar matriz de roles, permisos y logs de auditoría inmutables.

* **F2-01 (L) — Aislamiento Multi-tenancy en `rustem-db`:**
  * Contexto de sesión en Axum que inyecta `tenant_id` (Company ID) en el Request Extensions.
  * Wrapper en `rustem-db` que adjunta automáticamente `tenant_id` en cada consulta `SELECT`, `CREATE`, `UPDATE` o `RELATE`.
  * *Entregable:* Pruebas de integración HTTP verificando que el Tenant A no puede acceder a datos del Tenant B bajo ningún escenario.

* **F2-02 (L) — Matriz RBAC (Roles & Permisos):**
  * Definición de roles (`Owner`, `Administrador`, `Contador`, `Vendedor`, `Almacenista`).
  * Uso de relaciones de Grafo en SurrealDB (`user -> HAS_ROLE -> role -> CAN_ACCESS -> permission`).
  * Middleware en Axum (`RbacGuard`) para proteger rutas según el módulo y acción.
  * *Entregable:* Acceso denegado (HTTP 403 / Redirección) para usuarios sin el permiso requerido.

* **F2-03 (M) — Registro de Auditoría Inmutable (`audit_log`):**
  * Creación de aristas de grafo `RELATE user->EXECUTED->audit_log` para toda mutación de estado.
  * Captura de dirección IP (respetando proxy), timestamp UTC, tabla afectada, payload e ID del Tenant.
  * *Entregable:* Registro automático de auditoría en cada POST de la aplicación.

* **F2-04 (S) — Rate Limiter y Anti-Brute Force:**
  * Middleware Tower en Axum para mitigar ataques de fuerza bruta en `/login` (bloqueo por IP y usuario tras 5 intentos fallidos).
  * *Entregable:* Test de integración verificando respuesta HTTP 429 / Cuenta Bloqueada.

---

### FASE 3: INVENTARIO, ALMACENES & KARDEX (VALORACIÓN VEN-NIF)
*Objetivo:* Gestión de existencias multi-almacén con valoración de inventarios bajo NIC 2 / VEN-NIF (FIFO / Promedio Ponderado AVCO).

* **F3-01 (M) — Almacenes y Registro de Productos:**
  * ABM de Almacenes y Productos con unidades de medida, categorías, códigos de barra y alícuotas de IVA aplicables.
  * *Entregable:* Formularios SSR para gestión de productos y stock mínimo.

* **F3-02 (L) — Movimientos de Inventario y Grafo de Trazabilidad:**
  * Relación `RELATE warehouse->HAS_STOCK->product` con control de lote y fecha de vencimiento.
  * Movimientos inmutables: Entradas, Salidas, Transferencias e Ajustes vía transacciones ACID en SurrealDB.
  * *Entregable:* Invariante de stock: Prohibido stock negativo a menos que esté explícitamente configurado.

* **F3-03 (L) — Algoritmos de Valoración FIFO y Promedio Ponderado (AVCO):**
  * Implementación en `rustem-inventory` de cálculo de costo de ventas e inventario final conforme a NIC 2.
  * *Entregable:* Kardex auditable por producto con reconstrucción histórica del costo.

---

### FASE 4: VENTAS, FACTURACIÓN SENIAT & IGTF
*Objetivo:* Ciclo completo de ventas adaptado a la legislación fiscal venezolana vigente (SENIAT, IGTF, Imprenta Digital / Control Fiscal).

* **F4-01 (M) — Directorio de Clientes y Configuración Fiscal:**
  * Registro de Clientes con RIF, condición IVA (Contribuyente Ordinario, Formal, Sujeto Pasivo Especial - SPE).
  * *Entregable:* Formulario de clientes con validación RIF y estatus de retención.

* **F4-02 (L) — Máquina de Estados: Cotización → Pedido → Factura:**
  * Flujo de estados con transiciones estrictas y validación en el dominio.
  * Control de Series y Números de Control Fiscal SENIAT por empresa (ej. `00-000001`).
  * *Entregable:* Conversión de cotizaciones a facturas manteniendo trazabilidad.

* **F4-03 (L) — Motor Fiscal Venezolano (IVA, IGTF & Tasa BCV):**
  * Cálculo de IVA (16% General, 8% Reducida, Exento).
  * Cálculo de IGTF (alícuota configurable para pagos en moneda extranjera/divisas).
  * Imputación de tasa de cambio oficial BCV del día con captura de historial.
  * *Entregable:* Desglose fiscal exacto en la factura con montos expressados en Bolívares (VES) y Dólares (USD).

* **F4-04 (M) — Plantilla de Impresión Fiscal HTML+CSS (@media print):**
  * Vista de factura imprimible que cumple con la Providencia SNAT/2011/00071.
  * *Entregable:* Vista accesible en navegador que genera PDF impecable al presionar "Imprimir" (Ctrl+P) sin necesidad de JavaScript.

---

### FASE 5: CONTABILIDAD BICONTABLE DOBLE (VEN-NIF)
*Objetivo:* Sistema contable en partida doble con reexpresión bimoneda (VES/USD), integración automática desde ventas e inventario, y estados financieros.

* **F5-01 (L) — Plan de Cuentas y Estructura VEN-NIF:**
  * Plan de cuentas parametrizable (Activo, Pasivo, Patrimonio, Ingresos, Costos, Gastos, Cuentas de Orden).
  * Múltiples niveles de jerarquía y marcas de operabilidad.
  * *Entregable:* Arbol contable renderizado vía HTML nativo (`<details>/<summary>`).

* **F5-02 (XL) — Motor de Asientos Contables Inmutables & Integración Automática:**
  * Garantía de Partida Doble en el dominio: $\sum \text{Débitos} = \sum \text{Créditos}$ exacto en centavos.
  * Generación automática de asientos contables desde Ventas (Cuentas por Cobrar, IVA Débito Fiscal, Ventas, IGTF) y Movimientos de Inventario.
  * *Entregable:* Asientos contables generados atómicamente en la misma transacción de la venta o movimiento.

* **F5-03 (L) — Libros Contables y Balances Bimoneda:**
  * Reportes: Libro Diario, Libro Mayor, Balance de Comprobación (Sumas y Saldos), Estado de Resultados y Balance General.
  * Ajuste por Diferencia de Cambio conforme a NIC 21 / BA VEN-NIF 2.
  * *Entregable:* Consultas analíticas en SurrealDB con respuestas en < 200 ms.

---

### FASE 6: ENDURECIMIENTO, SEGURIDAD EN REPOSO & PRUEBAS DE CARGA
*Objetivo:* Asegurar el sistema ante auditorías de seguridad, implementar cifrado defensivo, pruebas de estrés y backups automatizados en Podman.

* **F6-01 (M) — Cifrado en Reposo y Gestión de Secretos:**
  * Cifrado de campos sensibles (datos bancarios, tokens de integración) usando `chacha20poly1305` / `aes-gcm`.
  * Limpieza de claves criptográficas en memoria usando `zeroize` y `secrecy`.
  * *Entregable:* Datos en reposo ilegibles en caso de volcado de la base de datos.

* **F6-02 (M) — Infraestructura TLS Nativa y Contenedor Podman:**
  * Configuración de Axum con `rustls` (CERO OpenSSL nativo C).
  * `Podmanfile` multi-stage build optimizado en Fedora Rawhide con usuario no root.
  * *Entregable:* Imagen de Podman lista para producción ejecutando en entornos rootless.

* **F6-03 (L) — Batería de Pruebas "Guardián Cero JS", Cobertura y Carga:**
  * Ejecución de `cargo-llvm-cov` para validar cobertura $\ge 85\%$ en módulos de dominio.
  * Pruebas de estrés con `oha` contra los handlers del servidor.
  * Test sintáctico automatizado que valida la ausencia total de `<script` y `on*=` en la salida HTML.
  * *Entregable:* Reportes de rendimiento p95 < 200 ms y cero hallazgos en `cargo audit`.

---

## 2. DEFINITION OF DONE (DoD)

### DoD Global (Aplica a todas las fases):
1. **Compilación Limpia:** `cargo check` y `cargo build --release` finalizan sin advertencias (*zero warnings*).
2. **Linter Estricto:** `cargo clippy -- -D warnings` se ejecuta sin ningún hallazgo.
3. **Formato:** `cargo fmt --check` valida la conformidad con las reglas del workspace.
4. **Seguridad:** `cargo audit` reporta cero vulnerabilidades en el árbol de dependencias.
5. **Cero C/C++:** Verificación de que no existan librerías enlazadas dinámicamente mediante C-FFI o `sys-crates` con código C.
6. **Guardián Cero JS:** Batería de pruebas HTTP que confirma que el 100% del HTML servido no contiene scripts ni atributos de eventos dinámicos.
7. **Multi-Tenancy:** Pruebas de integración confirman aislamiento completo por `tenant_id`.

### DoD por Fase:
* **Fase 1:** Contenedor de SurrealDB corriendo en Podman, migraciones aplicadas correctamente, login/logout funcional con cookies seguras `httpOnly` y prueba de integración de auth en verde.
* **Fase 2:** Matriz RBAC verificada con test unitarios; `audit_log` registrando eventos en grafo de SurrealDB.
* **Fase 3:** Kardex de inventario cuadrado con valoración FIFO/AVCO; cero posibilidad de stock negativo no autorizado.
* **Fase 4:** Factura SENIAT emitida con número de control correlativo, cálculo exacto de IVA e IGTF, e impresión CSS lista.
* **Fase 5:** Invariante de partida doble comprobado ($\sum \text{Débitos} = \sum \text{Créditos}$); asientos automáticos generados atómicamente.
* **Fase 6:** Cobertura de código $\ge 85\%$ en dominio, `oha` reporta p95 < 200 ms, y contenedor Podman ejecutable de forma autónoma.

---

## 3. MATRIZ DE RIESGOS TÉCNICOS Y DE PRODUCTO

| Riesgo | Tipo | Impacto | Mitigación |
| :--- | :--- | :--- | :--- |
| **Complejidad de Interfaz sin JS** | Técnico | Alto | Utilizar de forma intensiva componentes HTML5 semánticos (`<details>`, `<dialog>`, inputs nativos) y estados CSS (`:checked`, `:target`). Redirecciones PRG para refrescar estado desde servidor. |
| **Rendimiento SSR en consultas complejas** | Técnico | Medio | Optimizar consultas e índices SCHEMAFULL en SurrealDB. Aprovechar Tokio async y almacenamiento en memoria para sesiones activas. |
| **Volatilidad en normativa fiscal Venezolana** | Producto | Alto | Aislar la lógica fiscal (alícuotas de IVA, IGTF, formatos SENIAT) en configuraciones del Tenant en la base de datos, evitando hardcodear valores en el dominio. |
| **Diferencias de Redondeo en Moneda Bimoneda (VES/USD)** | Técnico | Crítico | Representar todos los montos en `i64` (centavos) y usar `rust_decimal` para las tasas de cambio. Queda prohibido el uso de flotantes `f32`/`f64`. |
| **Aislamiento Multi-tenant deficiente** | Seguridad | Crítico | Implementar wrappers obligatorios en la capa de persistencia `rustem-db` que inyecten el `tenant_id` en cada consulta, respaldado por pruebas de integración automatizadas. |

---

## 4. DECISIONES ABIERTAS Y REQUERIMIENTOS DE APROBACIÓN

1. **Gestión de Tasas de Cambio BCV:**  
   * *Propuesta:* Módulo de actualización manual diaria por el usuario administrador o carga vía tarea programada (*cron* interno en Rust) que consuma el API/sitio oficial del BCV.

2. **Facturación Electrónica / Imprenta Digital SENIAT:**  
   * *Propuesta:* Generación de formato impreso fiscal estándar y archivo XML/JSON exportable conforme a las normativas de proveedores autorizados de imprenta digital en Venezuela.

3. **Manejo de Reexpresión por Inflación (BA VEN-NIF 7 / NIC 29):**  
   * *Propuesta:* Módulo para la carga mensual del INPC (Índice de Precios al Consumidor) del BCV para habilitar la reexpresión de estados financieros a moneda constante cuando la empresa lo requiera.

---

## 5. REVISIÓN DE ESPECIFICACIONES CLAVE Y LÍMITES DE SURREALDB

* **Transacciones ACID:** En SurrealDB, las transacciones a través de bloques `BEGIN TRANSACTION ... COMMIT TRANSACTION` garantizan atomicidad en escrituras de múltiples tablas (asientos contables + balance + inventario).
* **Uso del Grafo (`RELATE`):** Se utilizará para rastrear auditoría (`user->EXECUTED->audit_log`) y trazabilidad de productos (`warehouse->HAS_STOCK->product`), manteniendo las consultas de contabilidad en estructuras tabulares SCHEMAFULL con índices para máxima velocidad.
* **Consultas Parametrizadas:** Todo el código en `rustem-db` hará uso de bindings del SDK oficial de Rust para prevenir cualquier tipo de inyección SurQL.

---

¿Aprobamos el plan y pasamos a la Fase 1?
