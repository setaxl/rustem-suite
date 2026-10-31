---
meta:
  contentType: Reference
  title: "¿Cómo planifico el desarrollo del ERP Rustem Suite?"
  navLabel: "Plan maestro Rustem Suite"
  category: "Requisitos y arquitectura"
---

# ¿Cómo planifico el desarrollo del ERP Rustem Suite?

TL;DR: este plan maestro ordena el desarrollo de Rustem Suite en seis fases (F1 a F6) con tareas acotadas, entregables verificables y una definición de hecho (DoD: Definition of Done) por fase. Cubre base y autenticación (F1), roles, aislamiento y auditoría (F2), inventario y valoración (F3), ventas con facturación fiscal, cobranza y pagos (F4), contabilidad en bimoneda, reexpresión y finanzas (F5) y endurecimiento con integraciones (F6), más los registros de decisión de arquitectura (ADRs), la matriz de riesgos y los límites técnicos de SurrealDB. La especificación técnica de cada módulo está en `especificaciones.md`.

El entorno de ejecución es Fedora Rawhide con Podman standard; el marco normativo lo conforman la Norma de Información Financiera venezolana (VEN-NIF) con el SENIAT (Providencia SNAT/2011/00071, IGTF, retenciones de IVA e ISLR, tasas del BCV); el frontend es Leptos SSR puro con cero JavaScript; y las dependencias son 100% Rust, sin librerías nativas de C o C++.

## 1. Backlog detallado por fases (F1 a F6)

Cada fase acumula la anterior; los tamaños entre paréntesis guían el dimensionamiento del sprint: S equivale a 1 día, M a 2 o 3 días, L a 4 o 7 días y XL a más de 8 días de esfuerzo.

### Fase 1: base e infraestructura core

En esta fase estableces el Cargo Workspace, la base de datos SurrealDB en Podman, las migraciones idempotentes, la arquitectura hexagonal y el flujo completo de autenticación con cookies seguras y SSR.

- **F1-01 (M): configuración del workspace y toolchain en Fedora Rawhide.**
  - Creas la estructura multi-crate (`erp-core`, `erp-auth`, `erp-admin`, `erp-db`, `erp-inventory`, `erp-sales`, `erp-purchasing`, `erp-accounting`, `erp-frontend`, `erp-web`).
  - Configuras el `Cargo.toml` raíz con edición 2024, Rust 1.99+ y perfiles de optimización.
  - Entregable: workspace ejecutable con `cargo check` y `cargo clippy` limpios.
- **F1-02 (L): capa de persistencia `erp-db` y SurrealDB 3.x en Podman.**
  - Scripting y ejecución de SurrealDB local con Podman en Fedora.
  - Sistema de migraciones idempotentes `.surql` embebidas con `rust-embed`.
  - Esquema base `SCHEMAFULL` para usuarios, sesiones, empresas e historial de tasas BCV.
  - Entregable: `erp-db` ejecutando consultas parametrizadas y aplicando el esquema al inicio.
- **F1-03 (L): crate `erp-core`, tipos de dominio, Money VEN-NIF y errores.**
  - Tipo `Money` con centavos en `i64`, código ISO 4217 (`VES`, `USD`) y operaciones exactas con `rust_decimal`.
  - Estructura del RIF venezolano (`V-`, `J-`, `G-`, `E-`, `P-`) con cálculo y validación del dígito verificador.
  - Entregable: suite de pruebas unitarias con `proptest` para la invariancia de redondeo y la validación de RIF.
- **F1-04 (L): autenticación segura (`erp-auth`).**
  - Hash de contraseñas con Argon2id (configuración ajustada a los estándares de OWASP 2026).
  - Tokens de sesión con `getrandom` (entropía de 256 bits) y almacenamiento del hash SHA-256 en SurrealDB.
  - Cookies `httpOnly`, `Secure` y `SameSite=Lax` emitidas por Axum.
  - Entregable: handlers de inicio de sesión, cierre, registro y recuperación con el flujo PRG (post-redirect-get).
- **F1-05 (M): vistas SSR con Leptos y layout CSS artesanal.**
  - Layout maestro HTML5 con Leptos SSR (cero JavaScript, sin `HydrationScripts`).
  - Estilos CSS artesanales con tokens de diseño (variables CSS) para modo claro y oscuro vía `:root` y `prefers-color-scheme`.
  - Entregable: páginas de inicio de sesión y registro respondiendo HTML nativo estilizado.

### Fase 2: RBAC, multi-tenancy y auditoría en grafo

En esta fase garantizas el aislamiento por empresa (tenant) e implementas la matriz de roles, permisos y logs de auditoría inmutables.

- **F2-01 (L): aislamiento multi-tenant en `erp-db`.**
  - Contexto de sesión en Axum que inyecta `tenant_id` (Company ID) en las extensiones del request.
  - Wrapper en `erp-db` que adjunta automáticamente el `tenant_id` a cada `SELECT`, `CREATE`, `UPDATE` o `RELATE`.
  - Entregable: pruebas de integración HTTP que verifican que el tenant A no accede a los datos del tenant B en ningún escenario.
- **F2-02 (L): matriz RBAC (roles y permisos).**
  - Definición de roles (`Owner`, `Administrador`, `Contador`, `Vendedor`, `Almacenista`).
  - Relaciones de grafo en SurrealDB: `user -> HAS_ROLE -> role -> CAN_ACCESS -> permission`.
  - Middleware `RbacGuard` en Axum para proteger rutas según módulo y acción.
  - Entregable: acceso denegado (HTTP 403 o redirección) para usuarios sin el permiso requerido.
- **F2-03 (M): registro de auditoría inmutable (`audit_log`).**
  - Aristas de grafo `RELATE user->EXECUTED->audit_log` para toda mutación de estado.
  - Captura de dirección IP (respetando el proxy), marca UTC, tabla afectada, payload e identificador del tenant.
  - Entregable: registro automático de auditoría en cada `POST` de la aplicación.
- **F2-04 (S): rate limiter y protección contra fuerza bruta.**
  - Middleware de Tower en Axum que mitiga ataques de fuerza bruta en `/login` (bloqueo por IP y usuario tras 5 intentos fallidos).
  - Entregable: prueba de integración que verifica la respuesta HTTP 429 y el bloqueo de cuenta.

### Fase 3: inventario, almacenes y kardex (valoración VEN-NIF)

En esta fase gestionas existencias multi-almacén con valoración conforme a la NIC 2 y al VEN-NIF (FIFO y promedio ponderado AVCO).

- **F3-01 (M): almacenes y registro de productos.**
  - ABM (alta, baja y modificación) de almacenes y productos con unidades de medida, categorías, códigos de barra y alícuotas de IVA aplicables.
  - Entregable: formularios SSR para la gestión de productos y stock mínimo.
- **F3-02 (L): movimientos de inventario y grafo de trazabilidad.**
  - Relación `RELATE warehouse->HAS_STOCK->product` con control de lote y fecha de vencimiento.
  - Movimientos inmutables: entradas, salidas, transferencias y ajustes vía transacciones ACID en SurrealDB.
  - Entregable: invariante de stock: el sistema rechaza el stock negativo salvo configuración explícita.
- **F3-03 (L): algoritmos de valoración FIFO y promedio ponderado (AVCO).**
  - Implementación en `erp-inventory` del cálculo del costo de ventas e inventario final conforme a la NIC 2.
  - Entregable: kardex auditable por producto con reconstrucción histórica del costo.

### Fase 4: ventas, facturación del SENIAT, cobranza y pagos

En esta fase implementas el ciclo completo de ventas ajustado a la legislación fiscal venezolana vigente (SENIAT, IGTF e imprenta digital o control fiscal), y añades las extensiones de cobranza (CxC), pagos (CxP), compras y tesorería.

- **F4-01 (M): directorio de clientes y configuración fiscal.**
  - Registro de clientes con RIF y condición IVA (Contribuyente Ordinario, Formal o Sujeto Pasivo Especial (SPE)).
  - Entregable: formulario de clientes con validación de RIF y estatus de retención.
- **F4-02 (L): máquina de estados de cotización a pedido a factura.**
  - Flujo de estados con transiciones validadas en el dominio.
  - Control de series y números de control fiscal del SENIAT por empresa (ejemplo `00-000001`).
  - Entregable: conversión de cotizaciones a facturas manteniendo la trazabilidad.
- **F4-03 (L): motor fiscal venezolano (IVA, IGTF y tasa BCV).**
  - Cálculo de IVA: 16% general, 8% reducida y bienes exentos.
  - Cálculo de IGTF con alícuota configurable para pagos en moneda extranjera.
  - Imputación de la tasa de cambio oficial del BCV del día con captura del historial.
  - Entregable: desglose fiscal exacto en la factura con montos expresados en bolívares (VES) y dólares (USD).
- **F4-04 (M): plantilla de impresión fiscal (HTML y CSS con `@media print`).**
  - Vista de factura imprimible conforme a la Providencia SNAT/2011/00071.
  - Entregable: vista en el navegador que produce una impresión impecable con Ctrl+P, sin JavaScript.
- **F4-05 (L): cuentas por cobrar (CxC) y gestión de deudores.**
- **F4-06 (L): compras, abastecimiento y facturación de proveedores.**
- **F4-07 (L): cuentas por pagar (CxP) con retenciones fiscales.**
- **F4-08 (L): tesorería y flujo de caja.**

### Fase 5: contabilidad bimoneda en partida doble (VEN-NIF)

En esta fase construyes el sistema contable en partida doble con reexpresión bimoneda (VES/USD), la integración automática desde ventas e inventario y los estados financieros. Además agregas a contabilidad los activos fijos, los presupuestos, los centros de costo y el BI.

- **F5-01 (L): plan de cuentas y estructura VEN-NIF.**
  - Plan de cuentas parametrizable: Activo, Pasivo, Patrimonio, Ingresos, Costos, Gastos y Cuentas de Orden.
  - Jerarquía multinivel y marcas de operabilidad.
  - Entregable: árbol contable renderizado con HTML native (`<details>` y `<summary>`).
- **F5-02 (XL): motor de asientos contables inmutables e integración automática.**
  - Garantía de partida doble en el dominio: la suma de los débitos es igual a la suma de los créditos, exacta en centavos.
  - Asientos automáticos desde ventas (cuentas por cobrar, IVA débito fiscal, ventas, IGTF) y movimientos de inventario.
  - Entregable: asientos generados atómicamente en la misma transacción de la venta o del movimiento.
- **F5-03 (L): libros contables y balances bimoneda.**
  - Reportes: libro diario, libro mayor, balance de comprobación (sumas y saldos), estado de resultados y balance general.
  - Ajuste por diferencia de cambio conforme a la NIC 21 y al BA VEN-NIF 2.
  - Entregable: consultas analíticas en SurrealDB con respuesta inferior a 200 ms.
- **F5-04 (L): integración contable de CxC.**
- **F5-05 (L): integración contable de CxP con retenciones.**
- **F5-06 (L): integración contable de tesorería.**
- **F5-07 (L): activos fijos y depreciación.**
- **F5-08 (M): presupuestos y control presupuesto vs. real.**
- **F5-09 (M): centros de costo y proyectos.**
- **F5-10 (M): BI y reportes gerenciales.**

### Fase 6: endurecimiento, seguridad en reposo y extensibilidad

En esta fase aseguras el sistema ante auditorías, implementas cifrado defensivo, pruebas de estrés, respaldos y las integraciones externas (API, importaciones, CRM avanzado, notificaciones).

- **F6-01 (M): cifrado en reposo y gestión de secretos.**
  - Cifrado de campos sensibles (datos bancarios, tokens de integración) con `chacha20poly1305` o `aes-gcm`.
  - Limpieza de claves criptográficas en memoria con `zeroize` y `secrecy`.
  - Entregable: datos en reposo ilegibles ante un volcado de la base de datos.
- **F6-02 (M): infraestructura TLS nativa y contenedor Podman.**
  - Configuración de Axum con `rustls` (sin OpenSSL nativo en C).
  - `Podmanfile` multi-stage optimizado para Fedora Rawhide, con la ejecución standard de Podman.
  - Entregable: imagen de Podman lista para producción, operable con `podman run` en modo estándar.
- **F6-03 (L): batería de pruebas guardián "cero JavaScript", cobertura y carga.**
  - Cobertura de 85% o más en módulos de dominio validada con `cargo-llvm-cov`.
  - Pruebas de estrés con `oha` contra los handlers del servidor.
  - Prueba sintáctica automatizada que valida la ausencia total de `<script` y `on*=` en el HTML de salida.
  - Entregable: reportes de rendimiento con p95 inferior a 200 ms y cero hallazgos en `cargo audit`.
- **F6-04 (M): API e integraciones externas.**
- **F6-05 (M): CRM avanzado y oportunidades.**
- **F6-06 (M): notificaciones y alertas.**
- **F6-07 (L): importaciones y aduanas (opcional).**

## 2. Definition of Done (DoD)

El DoD define los criterios de cierre: el global aplica a todas las fases, y el específico valida el cierre de cada fase.

### DoD global

1. **Compilación limpia:** `cargo check` y `cargo build --release` finalizan sin advertencias (zero warnings).
2. **Lint limpio:** `cargo clippy -- -D warnings` finaliza sin ningún hallazgo.
3. **Formato:** `cargo fmt --check` valida la conformidad con las reglas del workspace.
4. **Seguridad:** `cargo audit` reporta cero vulnerabilidades en el árbol de dependencias.
5. **Sin C/C++:** verificación de que no existan librerías enlazadas dinámicamente mediante C-FFI o `sys-crates` con código C.
6. **Guardián cero JavaScript:** batería de pruebas HTTP que confirma que el 100% del HTML servido no contiene scripts ni atributos de eventos dinámicos.
7. **Multi-tenancy:** pruebas de integración que confirman el aislamiento completo por `tenant_id`.

### DoD por fase

- **Fase 1:** contenedor de SurrealDB corriendo en Podman, migraciones aplicadas, inicio y cierre de sesión con cookies seguras `httpOnly` y pruebas de integración de auth en verde.
- **Fase 2:** matriz RBAC verificada con pruebas unitarias y `audit_log` registrando eventos en el grafo de SurrealDB.
- **Fase 3:** kardex de inventario cuadrado con valoración FIFO o AVCO y stock negativo bloqueado salvo configuración.
- **Fase 4:** factura del SENIAT emitida con número de control correlativo, desglose exacto de IVA e IGTF e impresión CSS lista; pagos de CxC, CxP y movimientos de tesorería con asientos automáticos.
- **Fase 5:** invariante de partida doble verificado (la suma de los débitos es igual a la suma de los créditos), asientos automáticos atómicos, libros bimoneda y reexpresión NIC 29 verificada.
- **Fase 6:** cobertura de 85% o más en dominio, `oha` reporta p95 inferior a 200 ms y el contenedor de Podman ejecuta el sistema de forma autónoma.

## 3. Matriz de riesgos técnicos y de producto

La matriz prioriza las mitigaciones; cada riesgo cubre el impacto y la medida correctiva concreta.

| Riesgo | Tipo | Impacto | Mitigación |
| :--- | :--- | :--- | :--- |
| Complejidad de interfaz sin JavaScript | Técnico | Alto | Intensificas el uso de HTML5 semántico (`<details>`, `<dialog>` e inputs nativos), estados CSS (`:checked`, `:target`) y redirecciones PRG para refrescar el estado desde el servidor. |
| Rendimiento SSR en consultas complejas | Técnico | Medio | Optimizas consultas e índices `SCHEMAFULL` en SurrealDB; aprovechas Tokio async y la memoria para las sesiones activas. |
| Volatilidad de la normativa fiscal venezolana | Producto | Alto | Aíslas la lógica fiscal (alícuotas de IVA, IGTF y formatos del SENIAT) en la configuración del tenant en la base de datos, sin valores fijados en el código del dominio. |
| Diferencias de redondeo en bimoneda (VES/USD) | Técnico | Crítico | Representas todos los montos en `i64` (centavos) y usas `rust_decimal` para las tasas de cambio; el dominio excluye los flotantes `f32` y `f64` para montos monetarios. |
| Aislamiento multi-tenant deficiente | Seguridad | Crítico | Implementas wrappers en `erp-db` que inyectan el `tenant_id` en cada consulta, con pruebas de integración automatizadas. |

## 4. Decisiones de negocio

Las tres open questions del backlog quedan resueltas; cada decisión adopta su opción elegida y queda registrada como un ADR (sección 6).

1. **Gestión de tasas de cambio BCV (DEC-001): opción híbrida.** El job programado (tokio-cron a las 10:00) realiza el consumo del sitio oficial del BCV cada día, el administrador corrige manualmente (overrides) cuando corresponde, el sistema alerta si falla la carga y registra la auditoría completa de cada cambio. Detalle en el ADR-007.
2. **Facturación impresa y electrónica (DEC-002): opción dual.** El formato HTML imprimible con CSS de impresión es el requisito vigente, y la exportación JSON/UBL 2.1 (Universal Business Language) queda configurable por tenant para los proveedores autorizados (FacturaYa, Sovos). Detalle en el ADR-008.
3. **Reexpresión por inflación (DEC-003): opción híbrida con motor.** El administrador carga el INPC mensual con validación, y el motor de reexpresión automático aplica la NIC 29 completa: coeficientes, clasificación monetaria, ajuste de patrimonio, resultado por posición monetaria y revelaciones. Detalle en el ADR-009.

## 5. Revisión de especificaciones clave y límites de SurrealDB

Estas notas de revisión guían el uso de SurrealDB para ajustar la implementación a sus límites reales.

- **Transacciones ACID:** los bloques `BEGIN TRANSACTION` y `COMMIT TRANSACTION` garantizan la atomicidad de las escrituras en múltiples tablas (asientos contables, balances e inventario).
- **Uso del grafo (`RELATE`):** el sistema emplea el grafo para trazar la auditoría (`user->EXECUTED->audit_log`) y la trazabilidad de productos (`warehouse->HAS_STOCK->product`), y mantiene las consultas de contabilidad en estructuras tabulares `SCHEMAFULL` con índices para la máxima velocidad.
- **Consultas parametrizadas:** el código de `erp-db` usa los bindings del SDK oficial de Rust para impedir la inyección de SurQL.

## 6. Decisiones de arquitectura (ADRs)

Los nueve ADRs registran las decisiones duraderas del sistema; el detalle completo vive en la sección 7 de `especificaciones.md` y aquí se listan resumidos para la trazabilidad del backlog.

- **ADR-001: SurrealDB sobre PostgreSQL.** Grafo con `RELATE`, esquema flexible y transacciones ACID; la curva de aprendizaje se mitiga con pruebas exhaustivas.
- **ADR-002: Leptos SSR puro sobre WebAssembly o SPA.** Cero JavaScript en el cliente, CSP sin `unsafe-inline`, SEO y conservación de la interactividad con HTML semántico y CSS.
- **ADR-003: Fedora Rawhide + Podman standard.** Motor de contenedores sin demonio y paquetes upstream; la rolling release exige pruebas frecuentes (CI en Fedora estable y Rawhide). Docker queda excluido.
- **ADR-004: Rust 100% puro, sin C/C++.** Solo crates 100% Rust (`rustls`, `ring`, RustCrypto, `rust-embed`); el sistema excluye OpenSSL, `libsqlite3` y `zlib` nativo.
- **ADR-005: `i64` en centavos + `rust_decimal`.** Precisión monetaria (NIC 21 y VEN-NIF) con aritmética verificada ante desbordes.
- **ADR-006: multi-tenancy con `tenant_id` inyectado por middleware.** Aislamiento garantizado por el wrapper de `erp-db` y pruebas de integración.
- **ADR-007: tasas BCV híbridas (automática, corrección manual, alertas).** Decide la DEC-001.
- **ADR-008: facturación dual (HTML imprimible + JSON/UBL configurable por tenant).** Decide la DEC-002.
- **ADR-009: reexpresión de inflación híbrida (INPC manual + motor automático NIC 29).** Decide la DEC-003.
