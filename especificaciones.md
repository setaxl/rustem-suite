---
meta:
  contentType: Reference
  title: "¿Cómo construyo un ERP modular 100% en Rust para Venezuela?"
  navLabel: "Especificaciones Rustem Suite"
  category: "Requisitos y arquitectura"
---

# ¿Cómo construyo un ERP modular 100% en Rust para Venezuela?

TL;DR: Rustem Suite es un plan de recursos empresariales (ERP: Enterprise Resource Planning) modular, con el 100% del código en Rust, arquitectura multi-empresa (multi-tenancy: un despliegue sirve a varias empresas con datos aislados) y adaptación al marco fiscal venezolano de 2026. Este documento de referencia define el stack tecnológico (Leptos en modo servidor, Axum, Tokio, SurrealDB orquestada con Podman standard), la arquitectura del workspace, los registros de decisión de arquitectura (ADRs: Architecture Decision Records), la especificación de cada módulo (cuentas por cobrar, cuentas por pagar, tesorería, activos fijos, presupuestos, centros de costo, compras, importaciones, gestión de relaciones con clientes (CRM), interfaz de programación de aplicaciones (API), notificaciones e inteligencia de negocios (BI)), los criterios de aceptación testeables, los ejemplos ejecutables y el glosario fiscal. Para el backlog de tareas por fase, consulta `plan_maestro.md`.

## 1. Rol y entornos de trabajo

Antes de escribir el primer crate (paquete de Rust) fijas quién construye el sistema y dónde: esta sección define el rol y los entornos de ejecución.

- Actúas como arquitecto de software senior especializado en el ecosistema Rust full-stack (Leptos en renderizado del servidor (SSR: server-side rendering), `Axum`, `Tokio`, `SurrealDB`) y en seguridad defensiva y criptografía aplicada, y eres copiloto técnico durante todo el ciclo de vida del proyecto.
- Ejecutas el desarrollo y despliegue en Fedora Rawhide.
- Orquestas contenedores efímeros y pruebas con **Podman standard** (motor de contenedores sin demonio, ejecución estándar). Docker queda excluido del proyecto.
- Gestionas la infraestructura auxiliar (ejecutable `surrealdb`, Geckodriver y Firefox para pruebas extremo a extremo (E2E)) con herramientas nativas de Fedora (`dnf`, `podman`).

> **Criterio de aceptación (entorno):** `podman info` funciona en Fedora Rawhide sin demonio externo, el repositorio solo contiene `Podmanfile` (cero `Dockerfile`) y `grep -ri "rootless" plan_maestro.md` devuelve 0 resultados.

## 2. Objetivo del sistema

El objetivo fija el alcance del producto; las secciones posteriores detallan ese alcance.

Construyes Rustem Suite, un plan de recursos empresariales (ERP) modular con el 100% del código en Rust, cuyo diseño toma la modularidad de Odoo como referencia. El sistema cubre contabilidad, ventas, inventario y administración, opera multi-empresa (cada tenant es la empresa registrada en el despliegue) y se adapta al marco comercial, fiscal y financiero de Venezuela vigente en 2026.

> **Criterio de aceptación (workspace):** `cargo check` sobre el workspace finaliza con 0 warnings.

## 3. Marco normativo fiscal y contable de Venezuela

El ERP implementa el marco contable y fiscal local; esta sección enumera las normas implementadas y te señala los detalles operativos.

### 3.1 Marco contable: VEN-NIF y bimoneda

- El sistema admite las dos variantes de la Norma de Información Financiera venezolana (VEN-NIF): la de Grandes Entidades (GE) y la de Pequeñas y Medianas Empresas (PYME); la configuración del tenant define la variante aplicada.
- La contabilidad opera en régimen de doble moneda (bimoneda): registra e imputa asientos en la moneda de cuenta legal, el bolívar venezolano (VES, código de la norma ISO 4217), y mantiene valores en moneda funcional opcional (USD o EUR) conforme al Boletín de Aplicación BA VEN-NIF 2 y a la Norma Internacional de Contabilidad (NIC) NIC 21, que gobierna los efectos de las variaciones en las tasas de cambio de la moneda extranjera.

### 3.2 Marco tributario: requisitos del SENIAT vigentes en 2026

- **Impuesto al Valor Agregado (IVA):** el sistema calcula las alícuotas del 16% (general), 8% (reducida) y 15% (suntuaria), soporta bienes exentos y no sujetos, y computa retenciones del 75% o del 100% cuando la contraparte califica como Sujeto Pasivo Especial (SPE).
- **Impuesto a las Grandes Transacciones Financieras (IGTF):** el sistema calcula e imputa este gravamen en pagos con divisas, efectivo o criptoactivos, con alícuota configurable por tenant según la Providencia del Servicio Nacional Integrado de Administración Aduanera y Tributaria (SENIAT) vigente.
- **Numeración y control fiscal:** el sistema emite facturas, notas de débito, notas de crédito y guías de despacho bajo la Providencia administrativa SNAT/2011/00071 del SENIAT (reglas de numeración, número de control, series y formato del Registro Único de Información Fiscal (RIF) con prefijos `V-`, `J-`, `G-`, `E-`, `P-`).
- **Comprobantes de retención:** el sistema genera comprobantes de retención de IVA e Impuesto Sobre la Renta (ISLR) con numeración correlativa anual o mensual.
- **Tasas oficiales del BCV:** el administrador consulta y actualiza la tasa de cambio oficial diaria del Banco Central de Venezuela (BCV), fuente requerida para la facturación y la valoración diaria, y el módulo mantiene el historial por fecha y hora; la automatización híbrida, el respaldo y la auditoría del dato quedan documentados en el ADR-007 (decisión de negocio 1).
- **Facturación impresa y electrónica:** el formato impreso con CSS de impresión (`@media print`) es el requisito vigente, y la exportación en notación de objetos JavaScript (JSON) con esquema Universal Business Language (UBL) 2.1 queda configurable por tenant; los detalles están en el ADR-008 (decisión de negocio 2).

### 3.3 Reexpresión por inflación (NIC 29 / BA VEN-NIF 7)

Venezuela opera en hiperinflación, por lo que los estados financieros se reexpresan a moneda de poder adquisitivo constante. Cargas mensualmente el Índice Nacional de Precios al Consumidor (INPC) publicado por el BCV, y el motor de reexpresión ejecuta el procedimiento completo de la NIC 29 y el BA VEN-NIF 7, documentado en el ADR-009 (decisión de negocio 3):

1. Calcula el coeficiente de cada periodo como el resultado de dividir el INPC del cierre entre el INPC de la fecha de origen de cada saldo.
2. Clasifica cada cuenta como monetaria (efectivo, bancos y partidas por cobrar o pagar pactadas en moneda funcional, que no se reexpresan) o no monetaria (inventario, inmuebles y patrimonio, que se reexpresan con el coeficiente).
3. Ajusta el patrimonio y obtiene la ganancia o pérdida del poder adquisitivo, que ingresa al estado de resultados.
4. Genera las revelaciones requeridas en las notas: método aplicado, índice base, resultado por posición monetaria y efecto en el patrimonio.

> **Criterios de aceptación (NIC 29):** `cargo test inflation::coefficients` (INF-01) valida coeficientes con INPC 100, 105 y 110; `cargo test inflation::monetary_classification` (INF-02) clasifica `1101` Banco como monetaria y `1501` Mercancía como no monetaria; `cargo test inflation::restatement` (INF-03) valida la reexpresión de un capital de 1.000.000 VES con INPC 110 en 1.100.000 VES y 100.000 de resultado por posición monetaria; `cargo test inflation::disclosure` (INF-04) verifica las revelaciones en las notas. Los cuatro se ejecutan y pasan.

## 4. Frontend sin JavaScript (SSR puro)

La regla de frontend es una condición de entrega: el HTML que produce el servidor es el entregable final.

- **Restricción central:** cero JavaScript en el cliente (hecho crítico): no incluyes scripts propios, de frameworks ni WebAssembly ejecutable en el navegador. Ejecutas Leptos en modo SSR puro, sin `HydrationScripts`.
- El HTML generado excluye, sin excepciones: etiquetas `<script>`, referencias a JavaScript externo, atributos de eventos (`onclick`, `onchange`, `onsubmit`, `onload`), estilos en línea (`style="..."`) y librerías de cliente (HTMX, Alpine.js, jQuery, React, Vue).
- **Interactividad permitida (estándares web nativos):**
  - Formularios nativos `<form method="post">` hacia los handlers de Axum y Leptos con el patrón PRG (post redirect get): envías, rediriges y solicitas con `GET`, y muestras el mensaje flash renderizado en el servidor.
  - HTML semántico avanzado: `<details>` y `<summary>` para menús colapsables, `<form method="dialog">` para diálogos, y entradas nativas (`type="date"`, `type="number"`, `select`, `type="checkbox"`).
  - CSS reactivo puro: `:target`, `:checked`, `:hover`, `:focus-within` y `prefers-color-scheme` para pestañas, visibilidad de modales, menú de navegación y alternancia de modo claro u oscuro sin JavaScript.
  - Modales re-renderizados con peticiones del protocolo de transferencia de hipertexto (HTTP) con método `GET`, o con el estado del elemento `<details open>`.
  - Toda mutación de estado se ejecuta con una petición `POST` seguida de redirección.
- **Estilos:** escribes CSS artesanal (sin Tailwind, sin Bootstrap ni Bulma) en `/static/*.css` servido por `tower-http` o empaquetado con `rust-embed`, y defines tokens de diseño con propiedades personalizadas (custom properties).
- **Accesibilidad:** HTML5 semántico, `<label for>` asociado a cada `id` y foco teclado visible con `:focus-visible`. El requerimiento completo de las normas Web Content Accessibility Guidelines (WCAG) nivel 2.1 AA está en la sección 21 (aspectos transversales).

> **Criterio de aceptación (guardián):** `cargo test web::zero_js_guard` inspecciona la salida HTML de cada endpoint de presentación y falla si encuentra `<script`, `on*=` o `style=`.

## 5. Stack tecnológico (100% Rust)

El stack materializa las decisiones de arquitectura registradas en los ADR-002, ADR-003, ADR-004 y ADR-005; esta sección fija versiones, crates y reglas de seguridad.

### 5.1 Versiones del ecosistema (octubre de 2026)

- **Lenguaje:** Rust 1.99+ con edición 2024 en todo el workspace.
- **Servidor de aplicaciones:** `Axum` 0.8+ sobre Tokio, con middleware `tower` y `tower-http` para el enrutamiento.
- **Frontend:** Leptos versión estable 2026 en SSR puro (sin WebAssembly ni hidratación en el cliente).
- **Base de datos:** SurrealDB 3.x en modo independiente (standalone), siempre consumida por el SDK oficial (`surrealdb`).

El acceso a SurrealDB cumple estas reglas:

- Esquema `SCHEMAFULL` con `DEFINE TABLE`, `DEFINE FIELD` y `DEFINE INDEX`.
- Migraciones `.surql` versionadas, aplicadas de manera idempotente al arrancar el sistema.
- Modelado de grafo con `RELATE` donde aplica: auditoría, trazabilidad de movimientos de inventario y matriz de control de acceso basado en roles (RBAC).
- Transacciones con propiedades ACID (atomicidad, consistencia, aislamiento y durabilidad) para asientos contables y kardex de inventario (registro cronológico por producto).
- 100% de consultas parametrizadas con los bindings del SDK; el código nunca interpola cadenas directamente.

### 5.2 Autenticación y sesiones

- Sesiones propias persistidas en SurrealDB y cookies seguras (`httpOnly`, `Secure`, `SameSite=Lax`) emitidas por Axum.
- Tokens de sesión generados con 256 bits de entropía mediante `getrandom`; la base de datos guarda únicamente el hash del token (SHA-256 o BLAKE3), no el token.
- Conviertes contraseñas a hash Argon2id con los parámetros recomendados por el Open Worldwide Application Security Project (OWASP).
- Bloqueo temporal de cuenta y dirección IP tras 5 intentos fallidos consecutivos.

### 5.3 Cifrado de datos sensibles

- Datos sensibles en reposo: `aes-gcm` o `chacha20poly1305` de RustCrypto.
- Seguridad en la capa de transporte (TLS: Transport Layer Security) con `rustls`.
- Limpieza de memoria crítica con `zeroize` y `secrecy` para claves y secretos.
- Hash de integridad de archivos con `sha2` y `blake3` (`sha2` y `blake3` quedan reservados para integridad; el hash de contraseñas corresponde a Argon2id).

### 5.4 Tipos de datos críticos

- **Los montos usan `i64` en unidades menores (centavos de VES o USD, con código ISO 4217) y el dominio excluye `f64` y `f32` para dinero** (ver ADR-005).
- Tasas, impuestos y porcentajes se representan con `rust_decimal` (aritmética decimal exacta).
- `chrono` para fechas y horas: almacenamiento global en UTC en la base de datos y conversión a la zona `America/Caracas` en la capa de presentación.

### 5.5 Soporte auxiliar

- Errores: `thiserror` para dominios y módulos, `anyhow` para la aplicación y los handlers.
- Registro de eventos: `tracing` y `tracing-subscriber`.
- Validación de entradas: `validator`.
- Configuración: `config` y `dotenvy`.
- Serialización: `serde` y `serde_json`.

### 5.6 Cabeceras de seguridad HTTP

Axum emite desde su middleware: política de seguridad de contenido (CSP: Content Security Policy) con `default-src 'self'`, sin `unsafe-inline` ni `unsafe-eval` (viable por la ausencia total de JavaScript); cookies seguras; `X-Frame-Options: DENY`; `X-Content-Type-Options: nosniff`; `Referrer-Policy: strict-origin-when-cross-origin`.

### 5.7 Política Rust-first y exclusión de C/C++

- Las dependencias son crates 100% Rust: `rustls` en lugar de OpenSSL, `ring` y `RustCrypto` para criptografía, `rust-embed` para recursos.
- El sistema excluye librerías C o C++, bindings nativos `unsafe`, OpenSSL, `libsqlite3` y `zlib` nativo (ver ADR-004).
- Implementas criptografía con crates auditadas; nunca escribes algoritmos criptográficos a mano.
- Controlas la cadena de suministro de forma continua con `cargo audit`, `cargo clippy -- -D warnings`, `cargo fmt --check` y `cargo llvm-cov`.

> **Criterio de aceptación (sin C/C++):** `cargo tree` no muestra crates con sufijo `-sys` que enlacen código C, y `cargo audit` reporta 0 vulnerabilidades conocidas.

## 6. Arquitectura del sistema

La arquitectura separa dominio, aplicación, infraestructura y presentación, concentra el acceso a datos en un único crate y garantiza el aislamiento multi-tenant por middleware.

### 6.1 Estructura del workspace de Cargo

```text
rustem-suite/
├── crates/
│   ├── erp-core/        # Tipos de dominio compartidos, Money, RIF, errores
│   ├── erp-auth/        # Autenticación, Argon2id, sesiones y tokens
│   ├── erp-admin/       # Tenancy, roles, permisos (RBAC), usuarios y empresas
│   ├── erp-db/          # Único punto de contacto con SurrealDB + migraciones .surql
│   ├── erp-inventory/   # Inventario, productos, almacenes, FIFO y AVCO, kardex
│   ├── erp-sales/       # Ventas, cotizaciones, pedidos, facturas SENIAT, IGTF, CxC
│   ├── erp-purchasing/  # Compras, órdenes, recepciones, facturas de proveedor, CxP
│   ├── erp-accounting/  # Plan de cuentas, asientos, libro mayor, reexpresión NIC 29
│   ├── erp-frontend/    # Componentes y vistas Leptos (SSR puro)
│   └── erp-web/         # Servidor Axum: rutas, handlers, middlewares y estáticos
├── static/              # CSS base, tokens de diseño, fuentes e imágenes
├── migrations/          # Esquema e índices idempotentes en archivos .surql
└── Podmanfile           # Contenedor de producción construido en Fedora
```

### 6.2 Arquitectura hexagonal y aislamiento

- El dominio define entidades, invariantes y casos de uso; la aplicación orquesta; la infraestructura implementa; la presentación renderiza vistas según el patrón modelo-vista-controlador (MVC) en Leptos SSR.
- El único acceso a datos atraviesa `erp-db`; los demás módulos dependen de las interfaces (traits) que expone la capa de dominio y persistencia.
- Cada entidad y consulta lleva `tenant_id`: el middleware de Axum extrae el identificador de la empresa desde la sesión activa y lo inyecta de manera transparente en cada operación (ver ADR-006).
- Implementas cada módulo del ERP como un crate del workspace, con `erp-core` compartido por todos los crates.

> **Criterios de aceptación (arquitectura):** `cargo clippy -- -D warnings` termina en verde en todo el workspace, `cargo test tenancy::isolation` demuestra que el tenant A no lee datos del tenant B, y `cargo test web::zero_js_guard` mantiene el cero JavaScript.

## 7. Decisiones de arquitectura (ADR)

Los registros de decisión (ADRs) documentan las opciones tomadas; cada uno registra estado, contexto, decisión, consecuencias y enlaces.

- **ADR-001: SurrealDB sobre PostgreSQL.**
  - Estado: aceptada.
  - Contexto: el modelo requiere grafo (auditoría, trazabilidad, RBAC) combinado con esquema flexible y transacciones ACID.
  - Decisión: SurrealDB 3.x standalone. Aportes: `RELATE` nativo, `SCHEMAFULL` opcional, transacciones ACID y SDK Rust oficial.
  - Consecuencias: curva de aprendizaje y comunidad más pequeña que PostgreSQL; se mitiga con pruebas exhaustivas.
  - Enlaces: documentación en `surrealdb.com/docs`.
- **ADR-002: Leptos SSR puro.**
  - Estado: aceptada.
  - Contexto: la restricción funcional excluye JavaScript en el cliente (seguridad, SEO y rendimiento).
  - Decisión: Leptos en modo SSR puro, sin `HydrationScripts`; el HTML sale directo del servidor.
  - Consecuencias: sin interactividad del lado del cliente; se compensa con HTML semántico (`<details>`, `<dialog>`, formularios PRG) y CSS reactivo (`:checked`, `:target`, `prefers-color-scheme`).
- **ADR-003: Fedora Rawhide + Podman standard.**
  - Estado: aceptada.
  - Contexto: el motor de contenedores sin demonio reduce la superficie de ataque, y Fedora entrega paquetes upstream recientes con compatibilidad Rust.
  - Decisión: Fedora Rawhide como SO base y **Podman standard** para contenedores (Docker excluido).
  - Consecuencias: la rolling release exige pruebas frecuentes; el CI cubre Fedora estable y Rawhide.
- **ADR-004: Rust 100% puro, cero C/C++.**
  - Estado: aceptada.
  - Contexto: la seguridad de la cadena de suministro, la auditoría y la preparación para WebAssembly piden el fin de los bindings en C.
  - Decisión: solo crates 100% Rust (`rustls`, `ring`, RustCrypto, `rust-embed`); el sistema excluye OpenSSL, `libsqlite3` y `zlib` nativo.
  - Consecuencias: algunas crates tienen menos madurez; `cargo audit` y la revisión manual de dependencias críticas la compensan.
- **ADR-005: dinero en `i64` + `rust_decimal`.**
  - Estado: aceptada.
  - Contexto: la precisión monetaria de la NIC 21 y del VEN-NIF excluye los flotantes `f32` y `f64` para dinero.
  - Decisión: `Money` usa `i64` en centavos más el código de moneda ISO 4217; tasas, impuestos y porcentajes usan `rust_decimal`.
  - Consecuencias: los valores extremos pueden desbordar; la aritmética verificada (checked) y los límites del dominio lo controlan.
- **ADR-006: multi-tenancy con `tenant_id` inyectado por middleware.**
  - Estado: aceptada.
  - Contexto: el aislamiento total de datos por empresa es un requisito legal y de seguridad.
  - Decisión: el middleware de Axum extrae el `tenant_id` de la sesión y `erp-db` lo inyecta en todas las consultas.
  - Consecuencias: la complejidad de las consultas compuestas crece; el wrapper de `erp-db` y las pruebas de integración multi-tenant la controlan.
- **ADR-007: tasas BCV híbridas (decide la DEC-001).**
  - Estado: aceptada.
  - Contexto: la facturación y la valoración requieren la tasa oficial del BCV del día, y el sitio del BCV carece de API oficial.
  - Decisión: el job programado (tokio-cron diario a las 10:00) consume el sitio del BCV; el administrador corrige manualmente (override), el sistema alerta si la carga falla y audita cada cambio (quién, cuándo, valor anterior y nuevo).
  - Consecuencias: la disponibilidad del sitio del BCV condiciona la obtención automática; la última tasa válida, la alerta y la corrección manual la respaldan.
- **ADR-008: facturación dual (decide la DEC-002).**
  - Estado: aceptada.
  - Contexto: el formato impreso cumple la Providencia SNAT/2011/00071 hoy, y los proveedores autorizados de imprenta digital esperan archivos estructurados.
  - Decisión: el HTML imprimible (CSS `@media print`) es el requisito vigente, y la exportación JSON/UBL 2.1 queda configurable por tenant (FacturaYa, Sovos).
  - Consecuencias: el mantenimiento doble de las plantillas introduce duplicados; un motor de plantillas compartido (`askama` o `tera`) lo mitiga.
- **ADR-009: reexpresión por inflación híbrida (decide la DEC-003).**
  - Estado: aceptada.
  - Contexto: la hiperinflación venezolana exige reexpresión de estados financieros (NIC 29 / BA VEN-NIF 7), y el dato crítico (INPC) requiere control humano.
  - Decisión: el administrador carga el INPC mensual con validación, y el motor automático calcula coeficientes, clasificación monetaria, ajuste de patrimonio, resultado por posición monetaria y revelaciones en las notas.
  - Consecuencias: el algoritmo exige exactitud; las pruebas de propiedad con `proptest` y los casos de referencia lo verifican.

## 8. Cuentas por cobrar (CxC)

La gestión de deudores completa el ciclo de ventas: aquí queda la especificación del ciclo de facturación, pagos, aging y libros fiscales.

### 8.1 Alcance y decisiones

La tabla siguiente consolida las decisiones de alcance adoptadas para CxC.

| Aspecto | Decisión |
| :--- | :--- |
| Detalle de pagos | Total por factura y desglose por línea de factura |
| Métodos de pago | Predefinidos (efectivo, transferencia, tarjeta, cheque, criptoactivo y mixto) y configurables por tenant (cuenta contable, comisión y habilitación) |
| Cuotas y financiamiento | Contado, crédito a días fijos, cuotas fijas (n cuotas con interés opcional) y plan personalizado (fechas y montos variables por factura) |
| Integración bancaria | Registro manual e importación de extractos en valores separados por comas (CSV) o Excel |
| Reporte fiscal | Dashboard interno, libro VEN-NIF formal de CxC y exportación CSV o JSON (notación de objetos JavaScript) para auditoría |
| Moneda | Multi-moneda con la tasa del BCV; la diferencia de cambio se contabiliza conforme a la NIC 21 |

### 8.2 Modelo de datos

Las entidades del sub-ledger de clientes residen en `erp-core` y se materializan en migraciones `SCHEMAFULL`:

- `Customer` (cliente): RIF, razón social, condiciones de pago por defecto, límite de crédito y estatus (activo, bloqueado o moroso).
- `PaymentTerm` (condición de pago): tipo (contado, crédito o cuotas), días, descuento por pronto pago, interés moratorio y plan de cuotas opcional.
- `Invoice` (factura): extiende la factura de Fase 4 con `balance_pendiente`, `payment_term_id`, `fecha_vencimiento` y `status_cxc` (pendiente, parcial, pago, vencido o anulado).
- `Payment` (pago): tenant, cliente, fecha, monto total, moneda, tasa de cambio, método, referencia bancaria y estado (borrador, confirmado, conciliado o anulado).
- `PaymentAllocation` (aplicación de pago): pago, factura, monto aplicado y fecha (permite aplicar un pago a varias facturas de forma parcial).
- `PaymentMethod` (método de pago): código, nombre, tipo, cuenta contable, comisión, habilitación y alcance (`tenant_id` nulo significa método global).
- `InstallmentPlan` (plan de cuotas): factura, número de cuotas, frecuencia, monto, interés y fecha de la primera cuota.
- `Installment` (cuota individual): plan, número, vencimiento, capital, interés, total y estado (pendiente, pagado, vencido o refinanciado).
- `BankStatementImport` (importación bancaria): archivo, fecha de importación, estado, líneas procesadas y conciliadas.
- `AgingSnapshot` (foto de aging): fecha de corte, cliente, buckets 0 a 30, 31 a 60, 61 a 90 y más de 90 días, y total vencido.

### 8.3 Funcionalidades por fase

**Fase 4 (módulo de ventas):**

- Modelo de datos en `erp-core` y migraciones con grafos `RELATE customer->HAS_INVOICE->invoice` y `invoice->HAS_PAYMENT->payment`.
- Registro de pagos multi-método con aplicación automática o manual a facturas (FIFO por vencimiento más prioridad manual).
- Condiciones de pago por cliente (heredadas del valor por defecto y sobrescribibles por factura), descuento por pronto pago e interés moratorio automático.
- Planes de cuotas fijas y personalizadas con generación y reprogramación automáticas.
- Importación de extractos bancarios (CSV o Excel) con matching sugerido por monto, fecha y referencia, más confirmación manual.
- Dashboard de cartera: aging interactivo con buckets configurables, saldo por cliente, mayores deudores y flujo de caja proyectado a 30, 60 y 90 días.
- Lista de cobranza con filtros (vencidos, por vencer, morosos), promesas de pago y acciones de seguimiento.
- Reportes: cartera por cliente, resumen de aging, facturas por vencer, cobranza efectiva y diferencia de cambio de la cartera.

**Fase 5 (módulo de contabilidad):**

- Asientos automáticos: factura (débito `1105`, crédito `4110` más `2408`), pago (débito `1101`, crédito `1105`), diferencia de cambio (`5115`), interés moratorio (`4210`) y descuento por pronto pago (`5110`).
- Conciliación bancaria por reglas (monto exacto, fecha ± 2 días, referencia) más conciliación manual y ajuste de diferencias.
- Sub-ledger de clientes sincronizado con el mayor y con el invariante: la suma de los saldos de clientes es igual al saldo de la cuenta `1105`.
- Libro VEN-NIF formal de CxC con las columnas del formato oficial: fecha, comprobante, cliente, RIF, vencimiento, monto original, abonos, saldo, días vencido y bucket.
- Exportación de auditoría en CSV o JSON con el esquema de facturas, pagos, asignaciones, aging y asientos vinculados.

### 8.4 Criterios de aceptación

> **Criterios de aceptación (CxC):** `cargo test cxc::aging` (CXC-01) valida el aging de 1000 facturas mixtas; `cargo test cxc::payment_allocation` (CXC-02) aplica el pago de 5000 a 3 facturas por FIFO; `cargo test cxc::installments` (CXC-03) calcula el plan de 12 cuotas de 1000 con interés del 2% (total 12240); `cargo test cxc::bank_import` (CXC-04) importa un CSV de 500 líneas con 480 coincidencias automáticas; `cargo test cxc::ven_nif_report` (CXC-05) valida las columnas del libro oficial; `cargo test cxc::subledger_invariance` (CXC-06) cuadra el sub-ledger contra la cuenta `1105` al centavo; `cargo test cxc::multi_currency` (CXC-07) contabiliza la diferencia de cambio de una factura de 1000 USD pagada al 36.55.

### 8.5 Ejemplos ejecutables

Los ejemplos de este módulo viven en el apéndice de ejemplos (sección 24), ya definido como archivos copiables: `crates/erp-core/src/cxc/types.rs`, `crates/erp-db/migrations/002_cxc_schema.surql`, `crates/erp-sales/src/cxc/payment_service.rs`, `crates/erp-accounting/src/cxc/entries.rs` y `crates/erp-web/src/handlers/cxc.rs`.

### 8.6 Términos del módulo

El glosario (sección 25) define: aging, pronto pago, interés moratorio, diferencia de cambio de la cartera, conciliación bancaria, plan de cuotas, sub-ledger, provisión de cobros dudosos y factoring.

## 9. Cuentas por pagar (CxP)

La gestión de proveedores replica el nivel de detalle de CxC e incorpora las retenciones fiscales propias de las compras.

### 9.1 Alcance: mismo nivel de detalle que CxC

La tabla consolida las decisiones de alcance adoptadas para CxP.

| Aspecto | Decisión |
| :--- | :--- |
| Detalle de pagos | Total por factura y desglose por línea de factura (simétrico a CxC) |
| Métodos de pago | Los mismos de CxC, configurables por tenant |
| Cuotas y financiamiento | Contado, crédito a días fijos, cuotas fijas y plan personalizado |
| Integración bancaria | Registro manual e importación de extractos en CSV o Excel |
| Reporte fiscal | Dashboard interno, libro VEN-NIF formal de CxP y exportación CSV o JSON para auditoría |
| Retenciones | IVA (75% o 100% para el SPE), ISLR según tarifa e IGTF en pagos en divisas |
| Verificación de compras | Verificación cruzada de los 3 documentos (three-way matching): orden, recepción y factura |

### 9.2 Modelo de datos

Las entidades del sub-ledger de proveedores residen en `erp-core` y se materializan en migraciones `SCHEMAFULL`:

- `Supplier` (proveedor): RIF, razón social, condiciones de pago por defecto, cuenta contable por defecto y retenciones aplicables (IVA, ISLR, IGTF).
- `PurchaseOrder` (orden de compra): proveedor, fecha, líneas (producto, cantidad, precio e IVA) y estado (borrador, enviada, parcial, recibida, facturada o cerrada).
- `GoodsReceipt` (recepción de mercancía): orden, fecha, almacén, líneas con cantidad real, lote y vencimiento, y estado.
- `SupplierInvoice` (factura de proveedor): orden o recepción de origen, proveedor, fecha, vencimiento, número de control fiscal, líneas, IVA, retenciones y `status_cxp` (pendiente, parcial, pago, vencido, anulado o disputado).
- `SupplierPayment` (pago al proveedor): tenant, proveedor, fecha, monto, moneda, tasa, método, referencia y estado.
- `SupplierPaymentAllocation` (aplicación de pago): pago, factura de proveedor, monto aplicado y fecha.
- `WithholdingVoucher` (comprobante de retención): factura, tipo (IVA o ISLR), base, porcentaje, monto, número correlativo, fecha y estado.

### 9.3 Funcionalidades por fase

**Fase 4 (compras y pagos):**

- Modelo de datos en `erp-core` y migraciones con grafos `RELATE supplier->HAS_PO->purchase_order`, `purchase_order->HAS_RECEIPT->goods_receipt` y `goods_receipt->HAS_INVOICE->supplier_invoice`.
- Ciclo completo: orden de compra, recepción (parcial o total) y factura de proveedor con verificación cruzada de los 3 documentos (three-way matching).
- Retenciones automáticas de IVA, ISLR e IGTF al registrar la factura, con generación de comprobantes correlativos.
- Registro de pagos multi-método con aplicación automática o manual a facturas (FIFO por vencimiento más prioridad manual).
- Condiciones de pago por proveedor, descuento por pronto pago e interés moratorio.
- Importación de extractos bancarios con coincidencia sugerida para pagos a proveedores.
- Dashboard de CxP: aging de proveedores, próximos vencimientos y flujo de caja de salidas proyectado.
- Reportes: CxP por proveedor, aging, retenciones generadas, comprobantes y diferencia de cambio.

**Fase 5 (contabilidad):**

- Asientos automáticos: factura de proveedor (débito `61xx` más `2407`, crédito `2105`), pago (débito `2105`, crédito `1101`), retención de IVA (débito `2105`, crédito `2409`), retención de ISLR (débito `2105`, crédito `2410`) y diferencia de cambio (`5115`).
- Sub-ledger de proveedores sincronizado con el invariante: la suma de los saldos de proveedores es igual al saldo de la cuenta `2105`.
- Libro VEN-NIF formal de CxP y comprobantes de retención de IVA e ISLR en el formato del SENIAT.
- Exportación de auditoría en CSV o JSON con el esquema documentado.

### 9.4 Criterios de aceptación

> **Criterios de aceptación (CxP):** `cargo test cxp::three_way_matching` (CXP-01) detecta el faltante de 5 unidades en una orden de 100; `cargo test cxp::withholding_calc` (CXP-02) calcula la retención de 120 sobre una factura de 1000 con IVA de 160 para un SPE (75%), y la retención de ISLR del 2% (20); `cargo test cxp::payment_allocation` (CXP-03) aplica el pago de 5000 a 3 facturas por FIFO; `cargo test cxp::ven_nif_report` (CXP-04) valida las columnas del libro y los comprobantes; `cargo test cxp::subledger_invariance` (CXP-05) cuadra el sub-ledger contra la cuenta `2105` al centavo.

### 9.5 Ejemplos y términos

Los ejemplos del módulo viven en el apéndice (sección 24), y el glosario (sección 25) define: verificación cruzada de 3 documentos, comprobante de retención, retención de IVA a proveedores SPE y costo landed de importaciones.

## 10. Tesorería y flujo de caja

La tesorería consolida las cuentas bancarias, la conciliación, la posición diaria y el flujo proyectado para que el gerente decide con la caja real en la mano.

### 10.1 Alcance

| Aspecto | Decisión |
| :--- | :--- |
| Cuentas bancarias | Múltiples por tenant y multi-moneda (VES, USD, EUR) con saldo visible al momento |
| Conciliación | Manual, importación CSV o Excel, coincidencia por reglas (monto, fecha ± 2 días, referencia) y conciliación automática en Fase 6 |
| Flujo proyectado | Entradas esperadas (cobros de CxC) más salidas programadas (pagos de CxP), nómina e impuestos, a horizontes de 30, 60 y 90 días |
| Posición diaria | Saldos de apertura, movimientos del día y saldos de cierre por cuenta y moneda |
| Transferencias internas | Entre cuentas del tenant, multi-moneda con la tasa del BCV o la del mercado configurada |
| Cheques | Emisión, depósito, rechazo y posfechados, con estados en cartera, depositado, cobrado o rechazado |

### 10.2 Modelo de datos

- `BankAccount` (cuenta bancaria): banco, tipo (corriente o ahorro), moneda, número, cuenta contable vinculada y estado.
- `BankStatement` (extracto): cuenta, periodo, balance inicial y final, archivo de origen y estado.
- `BankStatementLine` (línea de extracto): fecha, descripción, monto, tipo (débito o crédito), referencia, estado de conciliación y vínculo al pago, transferencia o brecha identificada (discrepancia).
- `CashFlowProjection` (proyección de flujo): fecha de corte, horizonte, entradas esperadas, salidas programadas y saldo proyectado.
- `InternalTransfer` (transferencia interna): cuentas de origen y destino, montos, monedas y tasa aplicada.

### 10.3 Funcionalidades por fase (Fase 4 y Fase 5)

- Modelo de datos con migraciones y asientos automáticos de transferencias, cheques e intereses bancarios.
- Conciliación bancaria: importación, coincidencia sugerida por reglas, confirmación manual y asiento de ajuste por diferencias.
- Posición diaria de tesorería: saldos por cuenta y moneda, con el disponible real contra el contable.
- Flujo de caja proyectado de forma automática desde cobros de CxC, pagos de CxP, impuestos y nómina importada (enlace a la sección 20 para el alcance de nómina).
- Gestión de cheques: emisión, seguimiento, depósito, rechazo y contabilización de cada estado.
- Reportes: posición diaria, flujo real contra proyectado, conciliación pendiente y cheques en cartera.

### 10.4 Criterios de aceptación

> **Criterios de aceptación (tesorería):** `cargo test treasury::bank_reconciliation` (TSY-01) concilia un CSV de 200 líneas (180 automáticas, 15 sugeridas, 5 sin coincidencia); `cargo test treasury::cash_flow_projection` (TSY-02) calcula el flujo neto de 15 M a 30 días con cobros de 50 M, pagos de 30 M e impuestos de 5 M; `cargo test treasury::daily_position` (TSY-03) cuadra saldos contables con el banco más los ítems pendientes de conciliación; `cargo test treasury::check_lifecycle` (TSY-04) contabiliza los asientos de cada estado del cheque.

Los ejemplos del módulo viven en el apéndice (sección 24), y el glosario (sección 25) define posición de tesorería y conciliación pendiente.

## 11. Activos fijos y depreciación

El módulo de activos fija la vida contable de los inmuebles y equipos, calcula la depreciación, gestiona revaluaciones y bajas, y concilla la depreciación fiscal con la contable.

### 11.1 Alcance

| Aspecto | Decisión |
| :--- | :--- |
| Tipos de activos | Bienes muebles, inmuebles, equipos, vehículos, mejoras en bienes arrendados e intangibles (software, licencias) |
| Métodos de depreciación | Lineal, dígitos de los años, unidades producidas y componentes (NIC 16) |
| Revaluación | Sí, conforme a la NIC 16, con ajuste al superávit de revaluación en patrimonio |
| Bajas y ventas | Cálculo de ganancia o pérdida, asiento automático y reporte fiscal |
| Depreciación fiscal contra contable | Diferenciada (ley de ISLR contra NIC 16), con conciliación de pasivos y activos diferidos |
| Inventario físico | Planillas de conteo, toma física, conciliación y ajustes |

### 11.2 Modelo de datos

- `FixedAsset` (activo fijo): código, descripción, categoría, cuentas contables (activo, depreciación acumulada y gasto), fecha de adquisición, costo original, vida útil en meses, método, valor residual, proveedor, factura de origen y estado.
- `DepreciationRun` (corrida de depreciación): periodo, fecha de ejecución, estado (borrador o confirmado) y total.
- `DepreciationLine` (línea de depreciación): corrida, activo, cuota del mes, acumulada y valor neto en libros.
- `AssetRevaluation` (revaluación): fecha, valor anterior y nuevo, superávit y cuenta de patrimonio.
- `AssetDisposal` (baja o venta): fecha, tipo (venta, baja o donación), valor de venta, costo retirado, depreciación acumulada retirada y resultado.

### 11.3 Funcionalidades por fase (Fase 5 y Fase 6)

- Modelo de datos con migraciones, alta, baja y modificación de activos, y asignación de centro de costo (sección 13).
- Motor de depreciación mensual por lote con vista de vista previa (preview) y asiento masivo.
- Revaluación conforme a la NIC 16: índice oficial o valor de mercado, ajuste del activo, superávit en patrimonio y recalculo de la depreciación futura.
- Bajas y ventas con cálculo de ganancia o pérdida, asiento y reporte fiscal (ganancia ocasional de ISLR).
- Depreciación fiscal (ley de ISLR) contra contable (NIC 16) con conciliación de diferidos.
- Inventario físico con planillas, captura, conciliación y ajustes.
- Reportes: registro de activos, depreciación del mes, movimientos del periodo, revaluaciones, bajas y diferidos.

### 11.4 Criterios de aceptación

> **Criterios de aceptación (activos fijos):** `cargo test fixed_assets::depreciation_lineal` (FA-01) calcula 1500 mensuales para un activo de 100.000 a 5 años con residual de 10.000; `cargo test fixed_assets::revaluation` (FA-02) registra el superávit de 50.000 y recalcula la depreciación; `cargo test fixed_assets::disposal_gain` (FA-03) obtiene la ganancia de 10.000 al vender en 80.000 con costo de 100.000 y acumulada de 30.000; `cargo test fixed_assets::fiscal_vs_contable` (FA-04) calcula el diferido cuando las dos depreciaciones divergen.

Los ejemplos del módulo viven en el apéndice (sección 24), y el glosario (sección 25) define vida útil, valor residual, superávit de revaluación y diferido fiscal.

## 12. Presupuestos (presupuesto contra ejecución real)

El módulo de presupuestos compara el plan aprobado contra la ejecución real y comprometida; alerta a los responsables antes del desvío, no después.

### 12.1 Alcance

| Aspecto | Decisión |
| :--- | :--- |
| Niveles | Empresa, centro de costo, proyecto y cuenta contable |
| Periodicidad | Anual con desglose mensual, ajustes trimestrales y reapertura anual |
| Tipos | Ingresos, gastos, inversiones (CapEx) y flujo de caja |
| Alertas | Umbrales de desviación: ámbar al 80%, rojo al 100% y verde bajo 80% |
| Versiones | Borrador, aprobado, ejecutando y cerrado, con histórico de versiones |
| Integración | Real desde el libro mayor; comprometido desde órdenes y pedidos de CxC y CxP |

### 12.2 Modelo de datos

- `Budget` (presupuesto): nombre, año, versión, estado y aprobador.
- `BudgetLine` (línea): presupuesto, cuenta contable, centro de costo, proyecto, mes y monto en la moneda del plan.
- `BudgetExecution` (ejecución): cuenta, centro, proyecto, mes, monto real, comprometido y disponible.
- `BudgetAlert` (alerta): línea, tipo (umbral de gasto o umbral de ingreso), porcentaje, fecha y estado de notificación.

### 12.3 Funcionalidades (Fase 5 y Fase 6)

- Carga del presupuesto de forma manual o por importación de Excel con plantilla.
- Ejecución automática mensual desde la contabilidad, más el comprometido desde órdenes y pedidos.
- Alertas automáticas por umbral con notificación de ello a los responsables (enlace a la sección 18, notificaciones).
- Reportes: presupuesto contra real (variación en monto y porcentaje), tendencia mensual, alertas activas y proyección del cierre.

### 12.4 Criterios de aceptación

> **Criterios de aceptación (presupuestos):** `cargo test budget::execution_auto` (BUD-01) actualiza `BudgetExecution` al registrar los asientos del mes; `cargo test budget::alerts` (BUD-02) emite la alerta ámbar y notifica al responsable al llegar al 90% del gasto.

Los ejemplos del módulo viven en el apéndice (sección 24).

## 13. Centros de costo y proyectos

La dimensión analítica habilita rentabilidad por área y por proyecto: imputa ingresos, costos y gastos más allá del plan de cuentas.

### 13.1 Alcance

- Centros de costo jerárquicos con estructura empresa, área y subárea.
- Proyectos con fecha inicial y final, presupuesto y responsable.
- Imputación analítica en asientos contables, facturas de venta y compra, nómina importada y depreciación.
- Rentabilidad por proyecto y centro, con ingresos contra costos directos e indirectos prorrateados.

### 13.2 Modelo de datos

- `CostCenter` (centro de costo): código, nombre, padre (jerarquía), responsable, cuenta de control y estado.
- `Project` (proyecto): código, nombre, fechas, presupuesto, centro asociado, responsable y estado.
- `AnalyticAllocation` (asignación analítica): asiento o línea de factura, centro, proyecto, porcentaje y monto.

### 13.3 Criterios de aceptación

> **Criterios de aceptación (centros de costo):** `cargo test cost_center::allocation` (CC-01) imputa un asiento de 10.000 al 60% en `CC01` y al 40% en `CC02` con líneas analíticas correctas; `cargo test project::profitability` (CC-02) calcula el margen de 20 M para un proyecto con ingresos de 100 M, costos directos de 60 M e indirectos de 20 M.

Los ejemplos del módulo viven en el apéndice (sección 24).

## 14. Compras y abastecimiento

El ciclo de compras extiende CxP e inventario con solicitudes, órdenes, recepción con control de calidad y la verificación cruzada de los 3 documentos.

### 14.1 Alcance

- Catálogo por proveedor con productos, servicios, precios acordados y condiciones comerciales.
- Solicitud de compra (PR: purchase requisition) con aprobación y conversión a orden.
- Orden de compra (PO: purchase order) con envío al proveedor, seguimiento y recepción parcial o total.
- Recepción de mercancía con control de calidad, lote, vencimiento y entrada automática al inventario.
- Factura de proveedor con verificación cruzada de los 3 documentos (sección 9) y retenciones (sección 9.3).
- Portal de proveedor opcional (Fase 6+): el proveedor consulta ordenes, carga facturas y revisa pagos.

### 14.2 Modelo adicional al de CxP

- `PurchaseRequisition` (solicitud): solicitante, centro de costo, líneas, aprobador y estado.
- `PurchaseOrderLine` (línea de orden): producto, cantidad, precio, fecha de entrega esperada, cantidad recibida y facturada.
- `QualityControl` (control de calidad): recepción, inspector, resultado (aprobado, rechazado o condicional) y observaciones.

### 14.3 Criterios de aceptación

> **Criterios de aceptación (compras):** `cargo test procurement::pr_to_po` (PRC-01) genera la orden con los datos de la solicitud aprobada; `cargo test procurement::three_way_match` (PRC-02) aprueba con orden de 100, recibo de 95 y factura de 95 (5 quedan pendientes de recepción); `cargo test procurement::inventory_integration` (PRC-03) registra la entrada del kardex al aprobar la recepción.

Los ejemplos del módulo viven en el apéndice (sección 24).

## 15. Importaciones y aduanas (opcional, Fase 6+)

El módulo de importaciones cubre el ciclo aduanal cuando el negocio importa mercancías; activas el módulo por decisión del dueño del producto al cerrar la Fase 5, y la tabla fija su alcance.

| Aspecto | Decisión |
| :--- | :--- |
| Licencia de importación (LI) | Número, fecha de emisión, fecha de vencimiento, cupo en USD y cupo usado |
| Declaración aduanera (DAU) | Número, fecha de registro, proveedor, valor CIF (costo, seguro y flete), arancel, IVA de importación e IGTF |
| Gastos de importación | Flete, seguro, agente, almacenaje y manipulación, integrados al costo landed del producto |
| Tasa aduanera | La tasa del BCV a la fecha de registro de la DAU contra la del pago, con la diferencia de cambio contabilizada |
| Inventario en tránsito | Control por puerto o almacén fiscal hasta la recepción final |

La mercancía importada requiere su propio costo: el sistema suma el valor CIF, el arancel, el IVA de importación, el IGTF y los gastos de llegada (costo landed) para valorizar el inventario por su costo total, y la tasa del BCV a la fecha de registro de la DAU contra la del pago genera la diferencia de cambio que la sección 9.3 contabiliza.

### 15.1 Modelo de datos

- `ImportLicense` (licencia de importación): número, fecha de emisión, fecha de vencimiento, cupo en USD, cupo usado y estado.
- `CustomsDeclaration` (declaración aduanera): licencia, número de DAU, fecha de registro, proveedor, total CIF en USD, arancel, IVA de importación, IGTF, tasa de la fecha de registro, gastos de importación en VES y costo landed en VES.
- `InTransitInventory` (inventario en tránsito): declaración, producto, cantidad, ubicación (puerto o almacén fiscal) y fecha estimada de llegada.

Los ejemplos del módulo viven en el apéndice (sección 24), y el glosario (sección 25) define DAU, costo landed e inventario en tránsito.

## 16. Gestión de relaciones con clientes (CRM avanzado)

El CRM amplía el cliente básico de la Fase 4 con el ciclo comercial completo; el embudo (pipeline) conecta la venta potencial del vendedor con la facturación del módulo de ventas.

### 16.1 Alcance

- Captura y calificación de prospectos (leads), con origen (formulario, campaña o referencia), puntaje de calificación (scoring) y asignación a vendedor.
- Oportunidades con etapas del embudo (nueva, calificada, propuesta, negociación, cerrada ganada o cerrada perdida), monto estimado, probabilidad, fecha de cierre estimada y competidor.
- Actividades de venta: llamadas, reuniones, correos, tareas y recordatorios, con resultado y próxima acción.
- Versiones de cotización por oportunidad con comparación, historial y conversión a pedido.
- Campañas de mercadeo con segmentación de clientes y registro de respuesta.
- Territorios y equipos de venta con metas por vendedor y por equipo.

### 16.2 Modelo de datos

- `Lead` (prospecto): origen, datos de contacto, puntaje, estado y vendedor asignado.
- `Opportunity` (oportunidad): prospecto o cliente, nombre, monto estimado, probabilidad, etapa, fecha de cierre estimada y competidor.
- `Activity` (actividad): oportunidad o cliente, tipo, fecha, descripción, resultado y próxima acción.
- `QuoteVersion` (versión de cotización): oportunidad, versión, fecha, líneas, total y estado (borrador, enviada, aceptada o rechazada).

La oportunidad cerrada ganada crea el pedido de ventas con las líneas de la cotización aceptada, y hereda las mismas reglas fiscales del motor de facturación (secciones 3.2 y 4). Los ejemplos del módulo viven en el apéndice (sección 24).

## 17. API externa e integraciones

La API expone las operaciones del ERP a terceros con la misma seguridad interna: autenticas cada llamada, limitas la tasa y auditas cada mutación.

### 17.1 Alcance

- API REST sobre los recursos principales (clientes, proveedores, productos, facturas, pagos y asientos) con operaciones de creación, lectura, actualización y listado (CRUD).
- Autenticación con OAuth 2.0 y claves de API por tenant con permisos granulares (scopes).
- Webhooks de eventos (factura emitida, pago recibido, stock mínimo, factura vencida) con entrega por `POST` firmado, reintentos con espera progresiva (backoff) y registro de cada entrega.
- Límite de tasa (rate limiting) por clave, endpoint y ventana de tiempo.
- Documentación OpenAPI 3.1 con la interfaz de prueba embebida, y generación de clientes (SDK) para TypeScript, Rust o Python desde la especificación.
- Cada respuesta respeta el aislamiento por tenant, y el versionado de la API lo fija la sección 21.3.

### 17.2 Modelo de datos

- `ApiKey` (clave de API): tenant, nombre, hash de la clave, permisos (scopes), límite de tasa en solicitudes por minuto, habilitación, vencimiento y hora del último uso.
- `WebhookEndpoint` (endpoint de webhook): tenant, URL, lista de eventos, secreto de firma, habilitación y reintentos máximos.
- `WebhookDelivery` (entrega de webhook): endpoint, evento, payload, código de respuesta, intentos y hora del próximo reintento.

## 18. Notificaciones y alertas

El módulo de notificaciones entrega cada evento del negocio por el canal correcto; los demás módulos notifican a través de esta sección.

### 18.1 Alcance

- Canales: correo saliente (SMTP), notificación dentro de la aplicación, webhook (sección 17) y mensaje corto (SMS) con proveedor configurable.
- Eventos: factura vencida, pago recibido, stock mínimo, alerta de presupuesto (sección 12), importación de datos completada, respaldo completado y error del sistema.
- Preferencias por usuario: canal, evento, horario de silencio y resumen diario o semanal (digest).
- Plantillas con variables dinámicas e idiomas (español por defecto, inglés y portugués preparados) conforme a la internacionalización (i18n) de la sección 21.4.
- Historial de entregas con estado (pendiente, enviada, fallida o leída) y reintento automático de las fallidas.

### 18.2 Modelo de datos

- `NotificationTemplate` (plantilla): código, nombre, canal, asunto, cuerpo de la plantilla e idioma.
- `NotificationPreference` (preferencia): usuario, evento, canal, habilitación y horario (inicio y fin del silencio).
- `Notification` (notificación): tenant, usuario, evento, canal, asunto, cuerpo, estado, hora de envío, hora de lectura e intentos.

## 19. Inteligencia de negocios (BI)

El BI convierte el dato operativo en decisión gerencial; los reportes aplican seguridad a nivel de fila (row-level security) según el rol, el centro de costo y el proyecto del usuario.

### 19.1 Alcance

- Paneles ejecutivos con indicadores clave de rendimiento (KPI: key performance indicators) de ventas, cobranza, rentabilidad, caja e inventario, con profundización por niveles (drill down).
- Creador de reportes para usuarios no técnicos, con exportación a Excel, PDF, CSV y JSON, y envío programado por correo con adjuntos (diario, semanal o mensual) vía la sección 18.
- Multi-moneda (VES, USD y EUR) con la tasa del BCV o la del mercado, seleccionable por reporte.
- Seguridad a nivel de fila por rol, centro de costo y proyecto.

### 19.2 Reportes clave

- Ventas: por vendedor, producto, cliente, canal y región, con tendencia y ticket promedio.
- Cobranza y pagos: aging, días promedio de cobro (DSO: days sales outstanding) y de pago (DPO: days payables outstanding), concentración de riesgo en los 10 mayores y efectividad de cobranza.
- Inventario: rotación, antigüedad del stock, rupturas de stock, valoración FIFO y AVCO, y análisis ABC.
- Financieros: flujo de caja, márgenes por línea, punto de equilibrio y comparativo del presupuesto (sección 12).
- Fiscales: IVA de compras y ventas, retenciones, IGTF y libros fiscales del SENIAT.

Los reportes consumen las vistas analíticas de la base y respetan los objetivos de latencia del SLA (sección 21.6).

## 20. Fuera del alcance del MVP

El MVP (producto mínimo viable) excluye dos dominios por decisión de producto; ambos quedan documentados con su integración prevista para que nadie los espere en el núcleo.

- **Nómina y recursos humanos:** integras proveedores especializados por la API externa (sección 17); el modelo mínimo en `erp-core` mantiene al empleado básico para la imputación contable, por centro de costo y por proyecto, y los asientos de nómina llegan importados en CSV desde el proveedor. La tesorería completa la proyección con ese dato importado (sección 10.3).
- **Producción y órdenes de trabajo:** la manufactura sale del núcleo mediante un módulo aparte o la integración con un sistema de ejecución de manufactura (MES); el modelo mínimo mantiene la orden de trabajo para el consumo de materiales hacia el inventario y la imputación de costos hacia la contabilidad.

## 21. Aspectos transversales

Las reglas de esta sección aplican a todos los módulos a la vez; los criterios de rendimiento (p95) complementan el objetivo general de la sección 22.

### 21.1 Cifrado por entidad

- Cifras en reposo con `aes-gcm` o `chacha20poly1305` los campos sensibles: usuario (hash de contraseña, correo y teléfono), cliente y proveedor (RIF y número de cuenta bancaria), cuenta bancaria (número de cuenta, clave SWIFT y token de la API del banco), pagos (referencia bancaria y token de tarjeta), claves de API (hash de la clave y secreto de firma del webhook), configuración fiscal (credenciales del proveedor de facturación) y `audit_log` (payload con datos personales).
- La limpieza de memoria (`zeroize`) protege las claves en uso, y ningún secreto llega al disco sin cifrar.

### 21.2 Auditoría de negocio

- Auditas creación, modificación, anulación y cambio de estado en cada entidad maestra, además de impresiones y exportaciones sensibles, inicios de sesión y fallos, y cambios de la configuración fiscal.
- Cada evento de auditoría guarda usuario, dirección IP de origen (con el proxy intermedio), user agent, marca UTC, entidad, identificador, acción, valores anteriores y nuevos, tenant e identificador de correlación del request.
- La retención cubre 7 años (plazo de conservación fiscal), con traslado a almacenamiento frío después del primer año.

### 21.3 Versionado

- Versionado semántico (SemVer) en formato mayor.menor.parche para el producto y para cada crate.
- Changelog en formato Keep a Changelog, generado desde los commits convencionales.
- Migraciones de base numeradas (`001_` a `004_`), idempotentes y aplicadas al arranque (sección 24).
- Versionado de la API con el encabezado `Accept: application/vnd.rustem.v1+json` y la ruta `/api/v1/`.

### 21.4 Internacionalización (i18n)

- Idiomas: español (por defecto), inglés y portugués, con claves de traducción (ejemplo `t!("invoice.created")`) y archivos por idioma (`.ftl` de Fluent o `.json`).
- Fechas y números con `chrono`, zona `America/Caracas` por defecto y configurable por tenant.
- Monedas con código ISO 4217 y formato conforme al idioma del usuario.

### 21.5 Accesibilidad (WCAG 2.1 AA)

- HTML semántico con landmarks, jerarquía de encabezados y etiquetas asociadas; los roles ARIA entran solo cuando el elemento nativo no basta.
- El contraste llega a la relación 4.5 a 1 en texto y 3 a 1 en elementos grandes (hecho crítico de accesibilidad), verificado por los tokens de diseño.
- Toda función opera con teclado (Tab, Enter, Esc y flechas), con foco visible con `:focus-visible`.
- Los mensajes flash y las alertas usan `aria-live`, y las imágenes llevan texto alternativo (`alt`).
- La batería de accesibilidad corre con `axe-core` en CI y con revisión manual de lectores de pantalla (NVDA y Orca).

### 21.6 Rendimiento y acuerdos de servicio

La tabla fija el objetivo de latencia p95, el rendimiento objetivo y las notas de cada módulo (SLA: service level agreement).

| Módulo | Objetivo p95 | Rendimiento objetivo | Notas |
| :--- | :--- | :--- | :--- |
| Autenticación | 500 ms | 100 solicitudes por segundo | Límite de tasa activo |
| Facturación | 800 ms | 50 solicitudes por segundo | Incluye la plantilla de impresión |
| Paneles de CxC y CxP | 300 ms | 200 solicitudes por segundo | Cache del aging de 5 minutos |
| Asientos contables | 200 ms | 500 solicitudes por segundo | Inserciones por lote |
| Reportes gerenciales | 2 s | 10 solicitudes por segundo | Transmisión de CSV en flujo (streaming) |
| Importación de CSV | 30 s por 10.000 filas | 5 trabajos concurrentes | Trabajo en segundo plano |

### 21.7 Capacidad y escalabilidad

| Límite | Valor | Estrategia |
| :--- | :--- | :--- |
| Tenants por instancia | 1.000 o más | Base compartida con aislamiento por `tenant_id` |
| Usuarios concurrentes por tenant | 500 o más | Pool de conexiones y servicios sin estado |
| Facturas por mes por tenant | 50.000 o más | Particionado por mes en SurrealDB |
| Líneas por factura | 500 | Validación en el dominio y en la base |
| Asientos por mes | 100.000 o más | Proceso por lotes e índices ajustados |

### 21.8 Licenciamiento y tenancy comercial

- Planes: Starter (1 empresa y 5 usuarios), Professional (5 empresas y 25 usuarios) y Enterprise (sin límite de empresas, un solo inicio de sesión (SSO) e integración por la API).
- Módulos opcionales activables: activos fijos, presupuestos, CRM, importaciones y API.
- Prueba de 30 días con todas las funciones; al vencer, el sistema baja al plan Starter y las notificaciones de la sección 18 lo comunican.

## 22. Estrategia de pruebas

La estrategia de pruebas protege los invariantes del negocio; cada capa verifica un aspecto distinto del sistema.

1. **Unitarias:** verificas invariantes contables (la suma de los débitos es igual a la suma de los créditos), los algoritmos FIFO y AVCO, el cálculo de IVA e IGTF, y la validación de RIF y numeración fiscal, con pruebas de propiedad de `proptest`.
2. **Integración (Podman + SurrealDB real):** validas migraciones, transacciones ACID y aislamiento multi-tenant en un contenedor efímero de Podman.
3. **HTTP (utilidades de prueba de `Axum`):** verificas con `tower::ServiceExt::oneshot` las cookies `httpOnly`, los flujos PRG, los códigos HTTP y las cabeceras de seguridad.
4. **Guardián "cero JavaScript":** automatizas una prueba que inspecciona el HTML de todos los endpoints de presentación y falla al detectar `<script` o atributos que inicien con `on*`.
5. **E2E (Fantoccini + navegador headless nativo en Fedora):** automatizas los flujos completos: inicio de sesión, creación de empresa, alta de inventario, venta con IGTF e IVA, y chequeo del asiento contable generado.
6. **Seguridad y rendimiento:** inspeccionas `cargo audit`, sometes el endpoint de inicio de sesión a pruebas de estrés con `oha` y mides con `criterion`; objetivo de p95 por debajo de 200 ms en handlers clave.

## 23. Criterios de aceptación testeables

Los 37 criterios consolidan las decisiones de negocio y los módulos; cada uno pasa o falla y ninguno admite cumplimientos parciales. La definición de hecho (DoD) de `plan_maestro.md` exige el total en verde al cierre.

### 23.1 Tasas del BCV, facturación dual y reexpresión (BCV, INV, INF)

| Código | Prueba | Resultado esperado |
| :--- | :--- | :--- |
| BCV-01 | `cargo test bcv::auto_fetch` | El job toma la tasa del BCV del día (36.50 VES/USD contra un servidor de prueba) y la guarda con fuente automática |
| BCV-02 | `cargo test bcv::manual_override` | El administrador corrige la tasa a 36.55 y la auditoría registra usuario, hora, valor anterior y valor nuevo |
| BCV-03 | `cargo test bcv::fallback` | Tras 3 días de fallo del sitio del BCV, el sistema usa la última tasa válida y activa la alerta |
| INV-01 | `cargo test invoice::html_print` | La factura impresa con CSS `@media print` reproduce el modelo del SENIAT |
| INV-02 | `cargo test invoice::export_json` | La factura exportada en JSON valida el esquema UBL 2.1 (`xmllint --schema`) |
| INV-03 | `cargo test invoice::tenant_config` | El tenant A genera solo HTML y el tenant B genera HTML más JSON según su configuración |
| INF-01 | `cargo test inflation::coefficients` | Con INPC 100, 105 y 110 los coeficientes salen 1.00, 1.05 y 1.10 |
| INF-02 | `cargo test inflation::monetary_classification` | La cuenta `1101` Banco queda monetaria y la `1501` Mercancía queda no monetaria |
| INF-03 | `cargo test inflation::restatement` | El capital de 1.000.000 VES con INPC 110 se reexpresa en 1.100.000 VES con 100.000 de resultado por posición monetaria |
| INF-04 | `cargo test inflation::disclosure` | Las notas revelan método, índice base, resultado por posición monetaria y efecto en el patrimonio |

### 23.2 Cuentas por cobrar (CXC)

| Código | Prueba | Resultado esperado |
| :--- | :--- | :--- |
| CXC-01 | `cargo test cxc::aging` | El aging de 1.000 facturas mixtas cuadra con la consulta de referencia |
| CXC-02 | `cargo test cxc::payment_allocation` | Un pago de 5.000 aplicado por FIFO a 3 facturas deja los saldos correctos |
| CXC-03 | `cargo test cxc::installments` | Las 12 cuotas de 1.000 con interés del 2% arrojan un total de 12.240 y fechas correctas |
| CXC-04 | `cargo test cxc::bank_import` | Un CSV de 500 líneas produce 480 coincidencias automáticas, 20 sugeridas y 0 errores de lectura |
| CXC-05 | `cargo test cxc::ven_nif_report` | El libro VEN-NIF sale con las columnas exactas y los totales cuadrados |
| CXC-06 | `cargo test cxc::subledger_invariance` | El sub-ledger de clientes cuadra con la cuenta `1105` al centavo (enteros `i64`) |
| CXC-07 | `cargo test cxc::multi_currency` | Una factura de 1.000 USD a 36.50 pagada a 36.55 contabiliza 50 VES de diferencia de cambio |

### 23.3 Cuentas por pagar (CXP)

| Código | Prueba | Resultado esperado |
| :--- | :--- | :--- |
| CXP-01 | `cargo test cxp::three_way_matching` | Con orden de 100, recepción de 95 y factura de 95, el faltante de 5 queda pendiente de recepción |
| CXP-02 | `cargo test cxp::withholding_calc` | La factura de 1.000 con IVA de 160 de un SPE retiene 120 (75%) y el ISLR del 2% retiene 20 |
| CXP-03 | `cargo test cxp::payment_allocation` | Un pago de 5.000 aplicado por FIFO a 3 facturas deja los saldos correctos |
| CXP-04 | `cargo test cxp::ven_nif_report` | El libro VEN-NIF de CxP y los comprobantes de retención salen con las columnas del SENIAT |
| CXP-05 | `cargo test cxp::subledger_invariance` | El sub-ledger de proveedores cuadra con la cuenta `2105` al centavo |

### 23.4 Tesorería (TSY)

| Código | Prueba | Resultado esperado |
| :--- | :--- | :--- |
| TSY-01 | `cargo test treasury::bank_reconciliation` | Un CSV de 200 líneas concilia 180 automáticas, 15 sugeridas y deja 5 sin coincidencia |
| TSY-02 | `cargo test treasury::cash_flow_projection` | Con cobros de 50M, pagos de 30M e impuestos de 5M, el flujo neto a 30 días es 15M |
| TSY-03 | `cargo test treasury::daily_position` | Los saldos contables cuadran con el banco más los ítems de conciliación pendientes |
| TSY-04 | `cargo test treasury::check_lifecycle` | Cada estado del cheque (emisión, depósito, cobro o rechazo) genera su asiento |

### 23.5 Activos fijos (FA)

| Código | Prueba | Resultado esperado |
| :--- | :--- | :--- |
| FA-01 | `cargo test fixed_assets::depreciation_lineal` | Un activo de 100.000 a 5 años con valor residual de 10.000 deprecia 1.500 por mes |
| FA-02 | `cargo test fixed_assets::revaluation` | El revalúo de 100.000 a 150.000 abre el superávit de 50.000 y recalcula la depreciación |
| FA-03 | `cargo test fixed_assets::disposal_gain` | La venta en 80.000 con costo de 100.000 y acumulada de 30.000 deja la ganancia de 10.000 |
| FA-04 | `cargo test fixed_assets::fiscal_vs_contable` | La divergencia fiscal contra contable abre el diferido correcto |

### 23.6 Presupuestos, centros de costo y compras (BUD, CC, PRC)

| Código | Prueba | Resultado esperado |
| :--- | :--- | :--- |
| BUD-01 | `cargo test budget::execution_auto` | Los asientos del mes actualizan la ejecución del presupuesto |
| BUD-02 | `cargo test budget::alerts` | El gasto al 90% genera la alerta ámbar y la notifica al responsable |
| CC-01 | `cargo test cost_center::allocation` | Un asiento de 10.000 se imputa 60% en `CC01` y 40% en `CC02` |
| CC-02 | `cargo test project::profitability` | Ingresos de 100M, costos directos de 60M e indirectos de 20M arrojan un margen de 20M |
| PRC-01 | `cargo test procurement::pr_to_po` | La solicitud aprobada genera la orden con los mismos datos |
| PRC-02 | `cargo test procurement::three_way_match` | Orden de 100, recibo de 95 y factura de 95 aprueban con 5 unidades pendientes |
| PRC-03 | `cargo test procurement::inventory_integration` | La recepción aprobada registra la entrada del kardex |

## 26. Plan del proyecto y reglas de trabajo

El proyecto avanza en fases acumulativas; esta sección resume la ruta y fija las reglas transversales de cumplimiento para cada tarea.

### 26.1 Ruta de fases

1. **Fase 0: plan maestro.** Documento `plan_maestro.md` con el backlog por fases, la definición de hecho (DoD: Definition of Done), la matriz de riesgos y los registros de decisión.
2. **Fase 1: base e infraestructura core.** Workspace de Cargo, SurrealDB en Podman, migraciones idempotentes, autenticación completa con cookies seguras y SSR, router `Axum` con CSS base y batería de pruebas.
3. **Fase 2: RBAC, multi-tenancy y auditoría.** Roles y permisos, asociación empresa y usuario, guarda de rutas en Axum, `audit_log` en grafo y limitador de inicio de sesión.
4. **Fase 3: inventario y valoración VEN-NIF.** Almacenes, productos, movimientos en grafo (`RELATE`), valoración primeras entradas, primeras salidas (FIFO: First In, First Out) y costo promedio ponderado (AVCO: average cost), y kardex inmutable.
5. **Fase 4: ventas, facturación SENIAT, cobranza y pagos.** Clientes, cotizaciones, pedidos, facturas con alícuotas de IVA e IGTF, correlativos de serie, formato impreso con `@media print`, y los flujos completos de cuentas por cobrar (CxC) y cuentas por pagar (CxP) con conciliación bancaria.
6. **Fase 5: contabilidad doble y bimoneda.** Plan de cuentas venezolano, asientos automáticos y manuales con invariante de partida doble en bimoneda, libro mayor, sumas y saldos, balance general, reexpresión NIC 29, activos fijos, presupuestos, centros de costo y BI.
7. **Fase 6: endurecimiento y extensibilidad.** Cifrado en reposo, respaldos con restauración validados por Podman, TLS nativo con `rustls`, pruebas de carga, API externa, importaciones, CRM avanzado y notificaciones.

El detalle de tareas, tamaños y entregables por fase está en `plan_maestro.md`.

### 26.2 Reglas de trabajo

- Comunicas el proyecto en español, incluidos los comentarios de código.
- Generas código completo y modular, sin fragmentos omitidos.
- Cada tarea finaliza con un commit de mensaje convencional (`docs:`, `feat:`, `fix:`, `test:`).
- El cierre de cada fase valida estos comandos en verde:

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt --check
cargo test
```
