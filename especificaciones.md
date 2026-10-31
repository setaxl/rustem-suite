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

## 22. Estrategia de pruebas

La estrategia de pruebas protege los invariantes del negocio; cada capa verifica un aspecto distinto del sistema.

1. **Unitarias:** verificas invariantes contables (la suma de los débitos es igual a la suma de los créditos), los algoritmos FIFO y AVCO, el cálculo de IVA e IGTF, y la validación de RIF y numeración fiscal, con pruebas de propiedad de `proptest`.
2. **Integración (Podman + SurrealDB real):** validas migraciones, transacciones ACID y aislamiento multi-tenant en un contenedor efímero de Podman.
3. **HTTP (utilidades de prueba de `Axum`):** verificas con `tower::ServiceExt::oneshot` las cookies `httpOnly`, los flujos PRG, los códigos HTTP y las cabeceras de seguridad.
4. **Guardián "cero JavaScript":** automatizas una prueba que inspecciona el HTML de todos los endpoints de presentación y falla al detectar `<script` o atributos que inicien con `on*`.
5. **E2E (Fantoccini + navegador headless nativo en Fedora):** automatizas los flujos completos: inicio de sesión, creación de empresa, alta de inventario, venta con IGTF e IVA, y chequeo del asiento contable generado.
6. **Seguridad y rendimiento:** inspeccionas `cargo audit`, sometes el endpoint de inicio de sesión a pruebas de estrés con `oha` y mides con `criterion`; objetivo de p95 por debajo de 200 ms en handlers clave.

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
