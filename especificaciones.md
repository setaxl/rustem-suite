# ESPECIFICACIONES TÉCNICAS Y REQUISITOS - RUSTEM SUITE (2026)

---

## 1. ROL Y ENTORNOS DE TRABAJO

* **ROL:** Arquitecto de Software Senior especializado en el ecosistema Rust full-stack (`Leptos SSR`, `Axum`, `Tokio`, `SurrealDB`) y en seguridad defensiva/criptográfica aplicada. Copiloto técnico activo durante todo el ciclo de vida del proyecto.
* **SISTEMA OPERATIVO & ORQUESTACIÓN:** 
  * Entorno de desarrollo/despliegue: **Fedora Rawhide**.
  * Orquestación de contenedores efímeros y pruebas: **Podman** (sin demonio, raíz/rootless nativo en Fedora). Se excluye de forma estricta Docker.
  * Si se requiere infraestructura auxiliar (ej. Podman-kube, `surrealdb` ejecutable o headless browser como Geckodriver/Firefox nativo en Fedora), se gestionará directamente con herramientas nativas de Fedora (`dnf`, `podman`).

---

## 2. OBJETIVO DEL SISTEMA

Construir **Rustem Suite**, un ERP modular 100% en Rust (inspirado en la modularidad de Odoo) que cubra contabilidad, ventas, inventario y administración, multi-empresa (*multi-tenancy*) y adaptado rigurosamente al marco comercial, fiscal y financiero de **Venezuela (2026)**.

---

## 3. MARCO NORMATIVO Y FISCAL VENEZOLANO (VEN-NIF & SENIAT - 2026)

El ERP se diseñará con adaptación contable y fiscal local:

1. **Marco Contable (VEN-NIF / BA VEN-NIF):**
   * **VEN-NIF GE** (Grandes Entidades) o **VEN-NIF PYME** según la configuración del Tenant.
   * **Contabilidad Bimoneda / Multimoneda activa:** Registro e imputación contable obligatoria en la moneda de cuenta legal (**Bolívares - VES**) y funcionalidad para moneda funcional opcional (**USD/EUR**) respetando la reconversión y reexpresión conforme a la **BA VEN-NIF 2** y **NIC 21** (Efectos de las variaciones en las tasas de cambio de la moneda extranjera).
   * **Reexpresión por Inflación (NIC 29 / BA VEN-NIF 7):** Preparación estructural para reexpresión basada en el Índice de Precios al Consumidor (INPC) publicado por el Banco Central de Venezuela (BCV).

2. **Marco Legal y Tributario (SENIAT / Leyes Vigentes):**
   * **Impuesto al Valor Agregado (IVA):** Soporte para Alícuota General (16%), Reducida (8%), Suntuaria (15%) y Exento/No Sujeto, con retenciones de IVA (75% / 100%) para Sujetos Pasivos Especiales (SPE).
   * **Impuesto Grandes Transacciones Financieras (IGTF):** Cálculo e imputación automática del IGTF (cobertura para transacciones en divisas/efectivo o cryptos, alícuotas configurables según Providencia Administrativa vigente).
   * **Numeración y Control Fiscal:** Emisión de Facturas, Notas de Débito, Notas de Crédito y Guías de Despacho bajo las reglas de la Providencia Administrativa **SNAT/2011/00071** (o Providencias de Facturación Electrónica/Imprenta Digital vigentes del SENIAT). Formato estricto para Números de Control, Serie y RIF (J-12345678-9, V-12345678-9, G-, E-, P-).
   * **Comprobantes de Retención:** Generación automática de comprobantes de retención de IVA e ISLR con numeración correlativa anual/mensual.
   * **Gestión de Tasas Oficiales:** Módulo de actualización e historial de tasas de cambio oficiales del **BCV** (Banco Central de Venezuela) fijadas por fecha y hora para la facturación y valoración diaria.

---

## 4. REGLA DURA DE FRONTEND: CERO JAVASCRIPT / SSR PURO

* **RESTRICCIÓN ABSOLUTA:** CERO JavaScript en el cliente. Ni scripts propios, ni scripts de frameworks, ni WebAssembly (`.wasm`) cargado en el navegador.
* **Leptos en Modo SSR Puro:** Sin `HydrationScripts`, sin ejecutable ejecutable client-side. El output HTML directo desde el servidor es el producto final de entrega.
* **PROHIBIDO en el HTML generado:**
  * Etiquetas `<script>` o referencias a JS externo.
  * Atributos de evento HTML (`onclick`, `onchange`, `onsubmit`, `onload`, etc.).
  * Estilos inline en los atributos HTML (`style="..."`).
  * Librerías cliente externas (tales como HTMX, Alpine.js, jQuery, React, Vue, etc.).
* **INTERACTIVIDAD PERMITIDA (Estándares Web Nativos):**
  * **Formularios:** Forms nativos HTML `<form method="post">` hacia los handlers de Axum/Leptos, aplicando de forma estricta el patrón **PRG (Post-Redirect-Get)** con mensajes flash de respuesta renderizados del lado del servidor.
  * **HTML Semántico Avanzado:** `<details>` y `<summary>` para menús desplegables/colapsables; `<form method="dialog">` para cuadros de diálogo; `<input type="date">`, `<input type="number">`, `<select>`, `<input type="checkbox">`.
  * **CSS Puro y Reactivo:** `:target`, `:checked`, `:hover`, `:focus-within`, `prefers-color-scheme` para pestañas, visibilidad de modales, menús de navegación y alternancia de modo oscuro/claro sin ejecución JS.
  * **Modales:** Re-renderizadas mediante peticiones GET dirigidas a la vista del servidor con atributo `open` o mediante el estado del elemento `<details>`.
  * **Acciones de Negocio:** Toda mutación de estado se ejecuta exclusivamente mediante peticiones HTTP `POST` redirigidas.
* **ESTILOS:** CSS puro artesanal (cero Tailwind CSS, cero Bootstrap/Bulma). Archivos CSS estáticos organizados en `/static/*.css` servidos vía `tower-http` o empaquetados con `rust-embed`. Uso de **CSS Custom Properties** (Design Tokens) para tematización y consistencia visual.
* **ACCESIBILIDAD:** HTML 5 semántico, etiquetas `<label>` explícitas asociadas mediante `for` a sus respectivos `id`, e indicadores visuales claros de foco (`:focus-visible`).

---

## 5. STACK TECNOLÓGICO Y POLÍTICA "100% RUST / ZERO C-C++ DEPENDENCIES"

### Versiones del Ecosistema (Actualizado a Octubre 2026):
* **Lenguaje:** **Rust 1.99+** (Edición 2024 activa por defecto en el workspace).
* **Backend Framework:** **Axum 0.8+** sobre **Tokio**, enrutamiento mediante middleware **Tower / Tower-HTTP**.
* **Frontend Framework:** **Leptos (última versión estable 2026)** utilizado en modo **SSR puro** (sin componente WASM/Hydration).
* **Base de Datos:** **SurrealDB 3.x / 2.6+ Standalone**, consumida vía SDK oficial en Rust (`surrealdb`).
  * Esquema **SCHEMAFULL** riguroso (`DEFINE TABLE`, `DEFINE FIELD`, `DEFINE INDEX`).
  * Migraciones `.surql` versionadas y aplicadas de forma idempotente al iniciar el sistema.
  * Modelado en Grafo mediante `RELATE` donde aplique (pistas de auditoría, trazabilidad de movimientos de inventario y matriz RBAC).
  * Transacciones ACID estrictas para asientos contables y kardex de inventario.
  * Consultas 100% parametrizadas (prohibida la interpolación directa de cadenas).
* **Autenticación & Sesiones:**
  * Sistema de sesiones propio cargado en SurrealDB.
  * Cookies seguras: `httpOnly`, `Secure`, `SameSite=Lax`.
  * Generación de tokens de sesión aleatorios de 256 bits mediante `getrandom`.
  * En Base de Datos se almacena **exclusivamente el Hash del token** (SHA-256 o BLAKE3).
  * Algoritmo de hashing de contraseñas: **Argon2id** (configurado según recomendaciones de parámetros OWASP).
  * Protección Anti-Fuerza Bruta: Bloqueo temporal de cuenta IP/Usuario tras 5 intentos fallidos consecutivos.
* **Cifrado y Seguridad Kryptos:**
  * Datos sensibles en reposo: `aes-gcm` o `chacha20poly1305`.
  * Comunicaciones TLS: `rustls`.
  * Limpieza de memoria crítica: `zeroize` y `secrecy` para claves y secretos.
  * Hash de integridad de archivos: `sha2` / `blake3` (prohibido su uso directo para contraseñas).
* **Tipos de Datos Críticos:**
  * **Dinero y Contabilidad:** `i64` en unidades menores (centavos de Bolívar/Dólar) combinado con el código de moneda ISO 4217 (`VES`, `USD`). **PROHIBIDO EL USO DE `f64` / `f32` PARA REPRESENTAR VALORES MONETARIOS**.
  * **Tasas, Impuestos y Porcentajes:** Uso obligatorio de la crate `rust_decimal`.
  * **Tiempo y Fechas:** `chrono` (Almacenamiento global en UTC dentro de la BD; conversión a zona horaria `America/Caracas` en la capa de presentación).
* **Soporte Auxiliar:**
  * Manejo de Errores: `thiserror` (dominio/módulos) + `anyhow` (aplicación/handlers).
  * Logging y Trazabilidad: `tracing` y `tracing-subscriber`.
  * Validación: `validator`.
  * Configuración: `config` y `dotenvy`.
  * Serialización: `serde` / `serde_json`.
* **Cabeceras de Seguridad HTTP (Axum Middleware):**
  * Content Security Policy (CSP): Estricta `default-src 'self'`. Sin `unsafe-inline` ni `unsafe-eval` (posible debido a la total ausencia de JavaScript).
  * Strict Cookies, `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`.

### Política Rust-First & Restricción C/C++:
* **ESTRICTAMENTE PROHIBIDO:** Uso de librerías basadas en C/C++ o bindings a librerías nativas `unsafe` en C (`OpenSSL`, `libsqlite3`, `zlib` nativo, etc.). 
* Se usará la alternativa **100% Rust pura** en todas las crates (`rustls` en lugar de OpenSSL, `ring`/`RustCrypto`, `rust-embed`).
* Prohibida la implementación manual o casera de algoritmos criptográficos.
* Garantía de Calidad Continua: `cargo-audit`, `cargo-clippy`, `cargo-fmt`, `cargo-llvm-cov`.

---

## 6. ARQUITECTURA DEL SISTEMA

### Estructura en Cargo Workspace (Multi-Crate):
```text
rustem-suite/
├── crates/
│   ├── rustem-core/        # Tipos de dominio compartidos, Money, VEN-NIF, errmod
│   ├── rustem-auth/        # Autenticación, Argon2id, sesiones y tokens
│   ├── rustem-admin/       # Tenancy, roles, permisos (RBAC), usuarios y empresas
│   ├── rustem-db/          # ÚNICO punto de contacto con SurrealDB + Migraciones .surql
│   ├── rustem-inventory/   # Inventario, productos, almacenes, FIFO/AVCO, Kardex
│   ├── rustem-sales/       # Ventas, cotizaciones, pedidos, facturas SENIAT, IGTF, retenciones
│   ├── rustem-accounting/  # Plan de cuentas, asientos, libro mayor, balance VEN-NIF
│   ├── rustem-frontend/    # Componentes y vistas Leptos (SSR puro)
│   └── rustem-web/         # Servidor Axum: rutas, handlers, middlewares y estáticos
├── static/              # Archivos CSS base, design tokens, fuentes e imágenes
├── migrations/          # Archivos idempotentes de esquema e índices .surql
└── Podmanfile           # Contenedor de producción/despliegue compilado en Fedora
```

### Arquitectura Hexagonal y Aislamiento:
* Dominio / Aplicación / Infraestructura / Presentación.
* **Acceso a Datos:** Exclusivamente desde `rustem-db`. Todos los módulos dependen de traits expuestos por la capa de dominio/persistencia.
* **Multi-tenancy:** Inyección obligatoria de `tenant_id` (Company ID) en **todas** las entidades y consultas. El *scope* se extrae e inyecta de forma transparente desde la sesión activa en el middleware de Axum.
* **Navegación:** Renderizado de vistas MVC mediante respuestas HTML directas de Leptos SSR.

---

## 7. PLAN DEL PROYECTO (FASE 0 A FASE 6)

* **Fase 0 — Plan Maestro:** Documento `PLAN_MAESTRO.md` (Backlog, DoD, matriz de riesgos, decisiones de diseño, revisión de capacidades SurrealDB).
* **Fase 1 — Base e Infraestructura:** Cargo Workspace, SurrealDB en Podman, migraciones, autenticación completa, Axum router + CSS base, batería de pruebas unitarias/integración.
* **Fase 2 — RBAC, Multi-tenancy & Auditoría:** Definición de roles y permisos, asociación empresa-usuario, guards de rutas en Axum, audit_log en grafo, rate limiter de login.
* **Fase 3 — Inventario & Valoración (VEN-NIF):** Almacenes, productos, movimientos en grafo (`RELATE`), valoración FIFO/AVCO, Kardex inmutable.
* **Fase 4 — Ventas, Retenciones & Facturación SENIAT:** Clientes, cotizaciones, pedidos, facturas con alícuotas IVA, IGTF, correlativos de serie SENIAT y formato impreso optimizado (HTML+CSS `@media print`).
* **Fase 5 — Contabilidad Doble & Bimoneda (VEN-NIF):** Plan de cuentas adaptado a Venezuela, asientos automáticos y manuales, partida doble e invariante bimoneda (VES/USD), libro mayor, sumas y saldos, balance general.
* **Fase 6 — Endurecimiento, Cifrado & Pruebas de Carga:** Cifrado en reposo, backups/restore validados con Podman, TLS nativo con `rustls`, pruebas de carga con `oha` y cobertura de código.

---

## 8. ESTRATEGIA DE PRUEBAS

1. **Unitarias:** Verificación de invariantes contables (débito == crédito), algoritmos de valoración FIFO/AVCO, cálculo de IVA e IGTF, validación de RIF y numeración fiscal. Pruebas de propiedad con `proptest`.
2. **Integración (Podman + SurrealDB real):** Ejecución en contenedor efímero de Podman. Validación de migraciones, transacciones ACID y aislamiento multi-tenant.
3. **HTTP (Axum Test Utils):** Verificación con `tower::ServiceExt::oneshot`. Comprobación de cookies `httpOnly`, flujos PRG, códigos HTTP y cabeceras de seguridad.
4. **Guardián "CERO JS":** Test automatizado que inspecciona el HTML resultante de todos los endpoints de presentación y **falla inmediatamente** si detecta las subcadenas `<script`, o atributos que inicien con `on*` (ej. `onclick=`).
5. **E2E (Fantoccini + Headless Browser nativo en Fedora):** Automatización de flujos completos: Login -> Crear Empresa -> Inventario -> Venta con IGTF/IVA -> Asiento Contable Automático.
6. **Seguridad y Rendimiento:** Inspección con `cargo audit`, tests de estrés contra el endpoint de login y benchmarking con `criterion` u `oha`.

---

## 9. REGLAS DE TRABAJO Y ENTREGABLES

* Idioma obligatorio de comunicación y comentarios de código: **Español**.
* Todo el código generado debe ser completo, modular, sin fragmentos omitidos (`...`) y libre de warnings de compilación o de `clippy`.
* Cierre de cada fase con la validación de los comandos:
  * `cargo check`
  * `cargo clippy -- -D warnings`
  * `cargo fmt --check`
  * `cargo test`
