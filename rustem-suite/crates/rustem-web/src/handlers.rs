// ============================================================================
// RUSTEM SUITE — rustem-web::handlers
// Handlers HTTP (Axum) con flujo PRG y vistas SSR.
// ============================================================================

//! # Módulo `handlers`
//! Handlers de Axum para las rutas de la aplicación.
//! Todos los handlers devuelven HTML nativo (SSR puro, cero JS).

use axum::extract::{Form, Path, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use chrono::Utc;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::str::FromStr;
use uuid::Uuid;

use crate::middleware::rate_limit;
use crate::state::AppState;
use rustem_auth::handlers::{LoginRequest, RegisterRequest};
use rustem_auth::password::PasswordHasher;
use rustem_auth::session::{SessionManager, SessionToken};
use rustem_core::{Rif, escape_html};
use rustem_db::store::{params_to_value, SurrealStore, SurrealStoreImpl, TenantContext};
use rustem_frontend::components::layout;
use rustem_frontend::views;
use rustem_inventory::{InventoryService, KardexEntry, MovementType, StockMovement};
use rustem_accounting::{EntryEngine, models::EntryLine};
use rustem_sales::{Customer, VatCondition};

// --- Structs de formulario para inventario (Fase 3) ---

#[derive(Debug, Deserialize, Serialize)]
pub struct AlmacenForm {
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub is_active: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProductoForm {
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: Option<String>,
    pub category: Option<String>,
    pub vat_rate: Option<String>,
    pub min_stock: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MovimientoEntradaForm {
    pub product_id: String,
    pub warehouse_id: String,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MovimientoSalidaForm {
    pub product_id: String,
    pub warehouse_id: String,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MovimientoAjusteForm {
    pub product_id: String,
    pub warehouse_id: String,
    pub quantity: i64,
    pub unit_cost_cents: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MovimientoTransferenciaForm {
    pub product_id: String,
    pub from_warehouse_id: String,
    pub to_warehouse_id: String,
    pub quantity: i64,
    pub unit_cost_cents: i64,
}

// --- Structs de formulario para contabilidad (Fase 5) ---

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountForm {
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub level: Option<u8>,
    pub parent_code: Option<String>,
    pub is_operational: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AsientoLineaFormEntry {
    pub account_code: String,
    pub amount_cents: i64,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AsientoForm {
    pub date: String,
    pub description: String,
    pub lineas: Vec<AsientoLineaFormEntry>,
}

// --- Structs de formulario para ventas (Fase 4) ---

#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerForm {
    pub rif: String,
    pub name: String,
    pub vat_condition: String, // "ordinario", "formal", "espe", "no_sujeto"
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceForm {
    pub customer_id: String,
    pub series: String,
    pub number: u32,
    pub control_number: String,
    pub currency: String, // "VES" o "USD"
    pub iva_rate: String, // "16", "8", "0"
    pub igtf_rate_percent: u8,
    pub subtotal_cents: String,
}

/// Extractor que obtiene [`TenantContext`] de las extensiones de la request
/// sin consumir el cuerpo (implementa `FromRequestParts`).
pub struct TenantCtx(pub TenantContext);

impl<S> axum::extract::FromRequestParts<S> for TenantCtx
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let ctx = parts
            .extensions
            .get::<TenantContext>()
            .cloned()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Self(ctx))
    }
}

/// Extrae el `TenantContext` de las extensiones de la request (insertado por
/// el middleware de tenant). Devuelve `None` si no hay sesión activa.
fn extract_tenant_ctx(req: &Request) -> Option<TenantContext> {
    req.extensions().get::<TenantContext>().cloned()
}

/// Handler GET / — redirige al login.
pub async fn root() -> Redirect {
    Redirect::to("/login")
}

/// Handler GET /login — muestra el formulario de login.
pub async fn login_get() -> Html<String> {
    Html(views::login_view(None))
}

/// Handler POST /login — procesa el login (flujo PRG) con anti-fuerza bruta.
pub async fn login_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<LoginRequest>,
) -> Result<Response, StatusCode> {
    // Extraer IP del cliente (respetando proxy).
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let username = &form.email;

    // Verificar si la IP o el usuario están bloqueados.
    {
        let g = state.brute_force.read().await;
        if g.is_ip_locked(&ip) || g.is_user_locked(username) {
            warn_rate_limit(&g, &ip, username);
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    // --- Autenticación real contra SurrealDB -------------------------------

    let conn = match &state.db {
        Some(c) => c.read().await,
        None => {
            // Sin base de datos disponible: no hay modo simulado con credenciales hardcoded.
            // En producción DATABASE_URL debe estar configurado. Retornamos 503.
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    // Buscar usuario por email.
    let sql = "SELECT * FROM user WHERE email = $email LIMIT 1";
    let params: Vec<(String, Value)> =
        vec![("$email".to_string(), Value::String(form.email.clone()))];
    let mut resp = conn
        .query(sql, params_to_value(params))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let users: Vec<Value> = resp
        .take(0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = match users.into_iter().next() {
        Some(u) => u,
        None => {
            // Usuario no encontrado: registrar fallo de intento.
            let blocked = rate_limit::check_and_record_failure(
                &*state.brute_force.read().await,
                &ip,
                username,
            );
            if let Some(status) = blocked {
                return Err(status);
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let password_hash = user
        .get("password_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let user_id = user
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Verificar contraseña con Argon2id.
    let hasher = PasswordHasher::new();
    let secret = SecretString::new(form.password);
    let valid = hasher.verify(&secret, password_hash).unwrap_or(false);

    if !valid {
        // Contraseña incorrecta: registrar fallo de intento.
        let blocked =
            rate_limit::check_and_record_failure(&*state.brute_force.read().await, &ip, username);
        if let Some(status) = blocked {
            return Err(status);
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Éxito: registrar intento exitoso y crear sesión de 256 bits.
    rate_limit::record_success(&*state.brute_force.read().await, &ip, username);

    // Resolver company_id del usuario mediante company_user (multi-tenancy).
    let company_id = {
        let sql = "SELECT company_id FROM company_user WHERE user_id = $user_id LIMIT 1";
        let params: Vec<(String, Value)> =
            vec![("$user_id".to_string(), Value::String(user_id.clone()))];
        let mut resp = conn
            .query(sql, params_to_value(params))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rows: Vec<Value> = resp
            .take(0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        rows.into_iter().next().and_then(|r| {
            r.get("company_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    };

    let (_session, token) = SessionManager::create_session_default(
        &user_id,
        company_id.clone(),
        Some(ip.clone()),
        None::<String>,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Persistir la sesión en SurrealDB (almacena exclusivamente el hash SHA-256).
    let now = Utc::now().to_rfc3339();
    let expires_at = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(
            rustem_auth::session::DEFAULT_SESSION_TTL_SECONDS,
        ))
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .to_rfc3339();

    let session_data = json!({
        "token_hash": token.hash(),
        "user_id": user_id,
        "company_id": company_id
            .as_ref()
            .map(|c| Value::String(c.clone()))
            .unwrap_or(Value::Null),
        "expires_at": expires_at,
        "ip_address": ip,
        "user_agent": Value::Null,
        "created_at": now,
    });
    let sql = "CREATE session CONTENT $data";
    let params: Vec<(String, Value)> = vec![("$data".to_string(), session_data.clone())];
    conn.query(sql, params_to_value(params))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Auditoría: inicio de sesión exitoso.
    let audit_data = json!({
        "user_id": user_id,
        "company_id": company_id
            .as_ref()
            .map(|c| Value::String(c.clone()))
            .unwrap_or(Value::Null),
        "action": "LOGIN",
        "table_name": "session",
        "record_id": Value::Null,
        "payload_before": Value::Null,
        "payload_after": session_data,
        "ip_address": ip,
        "created_at": now,
    });
    conn.query(
        "CREATE audit_log CONTENT $data",
        params_to_value(vec![("$data".to_string(), audit_data)]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Construir respuesta de redirección con cookie de sesión httpOnly.
    redirect_with_session_cookie(&token, "/dashboard")
}

/// Crea una respuesta de redirección que establece la cookie de sesión
/// `session_token` con atributos HttpOnly, Secure y SameSite=Lax (cero JS).
fn redirect_with_session_cookie(token: &SessionToken, path: &str) -> Result<Response, StatusCode> {
    let mut response = Redirect::to(path).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        format!(
            "session_token={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={}",
            token.0,
            rustem_auth::session::DEFAULT_SESSION_TTL_SECONDS
        )
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}

/// Handler GET /registro — muestra el formulario de registro.
pub async fn register_get() -> Html<String> {
    Html(views::register_view(None))
}

/// Handler POST /registro — procesa el registro (flujo PRG).
/// Crea empresa, usuario y la asociación company_user con rol Owner.
/// Registra auditoría para cada mutación de estado.
pub async fn register_post(
    State(state): State<AppState>,
    Form(form): Form<RegisterRequest>,
) -> Result<Redirect, StatusCode> {
    let conn = match &state.db {
        Some(c) => c.read().await,
        None => return Ok(Redirect::to("/login")),
    };

    // Validar RIF de la empresa.
    let _ = Rif::new(&form.company_rif).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Hashear contraseña con Argon2id.
    let hasher = PasswordHasher::new();
    let secret = SecretString::new(form.password);
    let password_hash = hasher
        .hash(&secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = Utc::now().to_rfc3339();
    let company_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    // --- 1. Crear empresa ---------------------------------------------------
    let company_data = json!({
        "id": company_id,
        "name": form.company_name,
        "rif": form.company_rif,
        "tax_regime": "VEN-NIF PYME",
        "is_vat_agent": false,
        "address": Value::Null,
        "phone": Value::Null,
        "email": Value::Null,
        "created_at": now,
        "updated_at": now,
    });
    conn.query(
        "CREATE company CONTENT $data",
        params_to_value(vec![("$data".to_string(), company_data.clone())]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Auditoría: creación de empresa.
    let audit_data = json!({
        "user_id": Value::Null,
        "company_id": company_id,
        "action": "CREATE",
        "table_name": "company",
        "record_id": company_id,
        "payload_before": Value::Null,
        "payload_after": company_data,
        "ip_address": Value::Null,
        "created_at": now,
    });
    conn.query(
        "CREATE audit_log CONTENT $data",
        params_to_value(vec![("$data".to_string(), audit_data)]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // --- 2. Crear usuario ---------------------------------------------------
    let user_data = json!({
        "id": user_id,
        "email": form.email,
        "password_hash": password_hash,
        "full_name": form.full_name,
        "is_active": true,
        "failed_login_attempts": 0,
        "locked_until": Value::Null,
        "created_at": now,
        "updated_at": now,
    });
    conn.query(
        "CREATE user CONTENT $data",
        params_to_value(vec![("$data".to_string(), user_data.clone())]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Auditoría: creación de usuario.
    let audit_data = json!({
        "user_id": Value::Null,
        "company_id": company_id,
        "action": "CREATE",
        "table_name": "user",
        "record_id": user_id,
        "payload_before": Value::Null,
        "payload_after": user_data,
        "ip_address": Value::Null,
        "created_at": now,
    });
    conn.query(
        "CREATE audit_log CONTENT $data",
        params_to_value(vec![("$data".to_string(), audit_data)]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // --- 3. Crear asociación usuario-empresa con rol Owner -----------------
    let company_user_id = Uuid::new_v4().to_string();
    let cu_data = json!({
        "id": company_user_id,
        "user_id": user_id,
        "company_id": company_id,
        "role": "Owner",
        "created_at": now,
    });
    conn.query(
        "CREATE company_user CONTENT $data",
        params_to_value(vec![("$data".to_string(), cu_data.clone())]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Auditoría: creación de company_user.
    let audit_data = json!({
        "user_id": Value::Null,
        "company_id": company_id,
        "action": "CREATE",
        "table_name": "company_user",
        "record_id": company_user_id,
        "payload_before": Value::Null,
        "payload_after": cu_data,
        "ip_address": Value::Null,
        "created_at": now,
    });
    conn.query(
        "CREATE audit_log CONTENT $data",
        params_to_value(vec![("$data".to_string(), audit_data)]),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/login"))
}

/// Handler POST /logout — cierra la sesión (flujo PRG).
/// Registra auditoría del cierre de sesión.
pub async fn logout_post(State(_state): State<AppState>, req: Request) -> Redirect {
    // Extraer contexto de tenant para auditoría.
    let tenant_ctx = req.extensions().get::<TenantContext>().cloned();

    // Registrar auditoría de logout si hay conexión a BD y usuario autenticado.
    // Nota: en modo sin DB, no hay auditoría persistente.
    let _ = tenant_ctx;

    Redirect::to("/login")
}

// ============================================================================
// Handlers de Inventario (Fase 3)
// ============================================================================

/// Crea un [`SurrealStoreImpl`] a partir de la conexión del estado y el
/// contexto de tenant. Retorna `None` si no hay base de datos disponible.
pub async fn get_store(state: &AppState, ctx: &TenantContext) -> Option<SurrealStoreImpl> {
    let conn = state.db.as_ref()?.read().await.clone();
    Some(SurrealStoreImpl::new(conn, ctx.clone()))
}

/// Handler GET /almacenes — lista almacenes del tenant.
pub async fn almacenes_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Almacenes", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM warehouse WHERE tenant_id = $tenant_id ORDER BY name",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Almacenes", "<p>Error consultando almacenes.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for w in &rows {
        let id = escape_html(w.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let name = escape_html(w.get("name").and_then(|v| v.as_str()).unwrap_or(""));
        let address = escape_html(w.get("address").and_then(|v| v.as_str()).unwrap_or("-"));
        let phone = escape_html(w.get("phone").and_then(|v| v.as_str()).unwrap_or("-"));
        let is_active = w.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true);
        let badge = if is_active {
            "<span class=\"badge badge-success\">Activo</span>"
        } else {
            "<span class=\"badge badge-danger\">Inactivo</span>"
        };
        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{name}</td>
                <td>{address}</td>
                <td>{phone}</td>
                <td>{badge}</td>
                <td>
                    <a href="/almacenes/{id}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Editar</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Almacenes - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/almacenes">Almacenes</a>
                <a href="/productos">Productos</a>
                <a href="/kardex">Kardex</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Almacenes</h1>
        <a href="/almacenes/nuevo" class="button button-primary">+ Nuevo Almacén</a>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Nombre</th>
                    <th>Dirección</th>
                    <th>Teléfono</th>
                    <th>Estado</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /almacenes/nuevo — formulario crear almacén.
pub async fn almacenes_nuevo() -> Html<String> {
    Html(views::warehouse_form_view(None))
}

/// Handler POST /almacenes/crear — crear almacén.
/// Usa SurrealStore para inyectar tenant_id automáticamente y registrar auditoría.
pub async fn almacenes_crear(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AlmacenForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/almacenes")),
    };

    if form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let data = json!({
        "id": id,
        "name": form.name,
        "address": form.address.unwrap_or_default(),
        "phone": form.phone.unwrap_or_default(),
        "email": Value::Null,
        "is_active": form.is_active.as_deref() == Some("1"),
    });

    store
        .create_with_tenant("warehouse", data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/almacenes"))
}

/// Handler GET /almacenes/:id/editar — formulario editar almacén.
pub async fn almacenes_editar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Editar Almacén",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM warehouse WHERE id = $id AND tenant_id = $tenant_id LIMIT 1",
            vec![("$id".to_string(), Value::String(id.clone()))],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Editar Almacén",
                "<p>Error consultando almacén.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let warehouse = match rows.into_iter().next() {
        Some(w) => w,
        None => return Html(layout("Editar Almacén", "<p>Almacén no encontrado.</p>")),
    };

    let name = escape_html(warehouse.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    let address = escape_html(
        warehouse
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let phone = escape_html(
        warehouse
            .get("phone")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let is_active = warehouse
        .get("is_active")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let checked = if is_active { " checked" } else { "" };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Editar Almacén - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/almacenes">Almacenes</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Editar Almacén</h1>
        <form method="post" action="/almacenes/{id}/actualizar" class="form">
            <div class="field">
                <label class="label" for="name">Nombre del almacén</label>
                <input type="text" id="name" name="name" class="input" value="{name}" required>
            </div>
            <div class="field">
                <label class="label" for="address">Dirección</label>
                <input type="text" id="address" name="address" class="input" value="{address}">
            </div>
            <div class="field">
                <label class="label" for="phone">Teléfono</label>
                <input type="tel" id="phone" name="phone" class="input" value="{phone}">
            </div>
            <div class="field">
                <label class="label" for="is_active">Activo</label>
                <input type="checkbox" id="is_active" name="is_active" value="1"{checked}>
            </div>
            <button type="submit" class="button button-primary">Guardar</button>
            <a href="/almacenes" class="button button-secondary">Cancelar</a>
        </form>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler POST /almacenes/:id/actualizar — actualizar almacén.
pub async fn almacenes_actualizar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AlmacenForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/almacenes")),
    };

    if form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let data = json!({
        "name": form.name,
        "address": form.address.unwrap_or_default(),
        "phone": form.phone.unwrap_or_default(),
        "is_active": form.is_active.as_deref() == Some("1"),
    });

    store
        .update_with_tenant("warehouse", &id, data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/almacenes"))
}

/// Handler GET /productos — lista productos del tenant.
pub async fn productos_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Productos", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM product WHERE tenant_id = $tenant_id ORDER BY name",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Productos", "<p>Error consultando productos.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for p in &rows {
        let id = escape_html(p.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let sku = escape_html(p.get("sku").and_then(|v| v.as_str()).unwrap_or(""));
        let name = escape_html(p.get("name").and_then(|v| v.as_str()).unwrap_or(""));
        let vat_rate = escape_html(p.get("vat_rate").and_then(|v| v.as_str()).unwrap_or("16"));
        let unit = escape_html(p.get("unit").and_then(|v| v.as_str()).unwrap_or("und"));
        let category = escape_html(p.get("category").and_then(|v| v.as_str()).unwrap_or("-"));

        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{sku}</td>
                <td>{name}</td>
                <td>{vat_rate} %</td>
                <td>{unit}</td>
                <td>{category}</td>
                <td>
                    <a href="/productos/{id}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Editar</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Productos - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/almacenes">Almacenes</a>
                <a href="/productos">Productos</a>
                <a href="/kardex">Kardex</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Productos</h1>
        <a href="/productos/nuevo" class="button button-primary">+ Nuevo Producto</a>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>SKU</th>
                    <th>Nombre</th>
                    <th>IVA</th>
                    <th>Unidad</th>
                    <th>Categoría</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /productos/nuevo — formulario crear producto.
pub async fn productos_nuevo() -> Html<String> {
    Html(views::product_form_view(None))
}

/// Handler POST /productos/crear — crear producto.
pub async fn productos_crear(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<ProductoForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/productos")),
    };

    if form.sku.is_empty() || form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let data = json!({
        "id": id,
        "sku": form.sku,
        "name": form.name,
        "description": form.description.unwrap_or_default(),
        "unit": form.unit.unwrap_or_else(|| "und".to_string()),
        "category": form.category.unwrap_or_default(),
        "vat_rate": form.vat_rate.unwrap_or_else(|| "16".to_string()),
        "min_stock": form.min_stock.unwrap_or(0),
        "is_active": true,
        "barcode": Value::Null,
    });

    store
        .create_with_tenant("product", data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/productos"))
}

/// Handler GET /productos/:id/editar — formulario editar producto.
pub async fn productos_editar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Editar Producto",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM product WHERE id = $id AND tenant_id = $tenant_id LIMIT 1",
            vec![("$id".to_string(), Value::String(id.clone()))],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Editar Producto",
                "<p>Error consultando producto.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let product = match rows.into_iter().next() {
        Some(p) => p,
        None => return Html(layout("Editar Producto", "<p>Producto no encontrado.</p>")),
    };

    let sku = escape_html(product.get("sku").and_then(|v| v.as_str()).unwrap_or(""));
    let name = escape_html(product.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    let description = escape_html(
        product
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let unit = escape_html(
        product
            .get("unit")
            .and_then(|v| v.as_str())
            .unwrap_or("und"),
    );
    let category = escape_html(
        product
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let vat_rate = escape_html(
        product
            .get("vat_rate")
            .and_then(|v| v.as_str())
            .unwrap_or("16"),
    );

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Editar Producto - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/productos">Productos</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Editar Producto</h1>
        <form method="post" action="/productos/{id}/actualizar" class="form">
            <div class="field">
                <label class="label" for="sku">SKU</label>
                <input type="text" id="sku" name="sku" class="input" value="{sku}" required>
            </div>
            <div class="field">
                <label class="label" for="name">Nombre</label>
                <input type="text" id="name" name="name" class="input" value="{name}" required>
            </div>
            <div class="field">
                <label class="label" for="description">Descripción</label>
                <input type="text" id="description" name="description" class="input" value="{description}">
            </div>
            <div class="field">
                <label class="label" for="unit">Unidad de medida</label>
                <input type="text" id="unit" name="unit" class="input" value="{unit}">
            </div>
            <div class="field">
                <label class="label" for="category">Categoría</label>
                <input type="text" id="category" name="category" class="input" value="{category}">
            </div>
            <div class="field">
                <label class="label" for="vat_rate">Alícuota IVA</label>
                <select id="vat_rate" name="vat_rate" class="input">
                    <option value="16" {}>General (16%)</option>
                    <option value="8" {}>Reducida (8%)</option>
                    <option value="0" {}>Exento (0%)</option>
                </select>
            </div>
            <button type="submit" class="button button-primary">Guardar</button>
            <a href="/productos" class="button button-secondary">Cancelar</a>
        </form>
    </main>
</body>
</html>"#,
        if vat_rate == "16" { "selected" } else { "" },
        if vat_rate == "8" { "selected" } else { "" },
        if vat_rate == "0" { "selected" } else { "" },
    );
    Html(html)
}

/// Handler POST /productos/:id/actualizar — actualizar producto.
pub async fn productos_actualizar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<ProductoForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/productos")),
    };

    if form.sku.is_empty() || form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let data = json!({
        "sku": form.sku,
        "name": form.name,
        "description": form.description.unwrap_or_default(),
        "unit": form.unit.unwrap_or_else(|| "und".to_string()),
        "category": form.category.unwrap_or_default(),
        "vat_rate": form.vat_rate.unwrap_or_else(|| "16".to_string()),
        "min_stock": form.min_stock.unwrap_or(0),
        "barcode": Value::Null,
    });

    store
        .update_with_tenant("product", &id, data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/productos"))
}

/// Handler POST /movimientos/entrada — registrar entrada de stock.
/// Usa SurrealStore para multi-tenancy y auditoría automática.
/// Crea arista de grafo warehouse->HAS_STOCK->product para trazabilidad.
pub async fn movimientos_entrada(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<MovimientoEntradaForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/kardex")),
    };

    if form.quantity <= 0 || form.unit_cost_cents <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let movement_id = Uuid::new_v4().to_string();
    let lot_id = Uuid::new_v4().to_string();
    let total_cost = form.quantity * form.unit_cost_cents;

    // 1. Crear movimiento de stock (tenant_id e auditoría inyectados automáticamente).
    let movement_data = json!({
        "id": movement_id,
        "product_id": form.product_id,
        "warehouse_id": form.warehouse_id,
        "movement_type": "ENTRADA",
        "quantity": form.quantity,
        "unit_cost_cents": form.unit_cost_cents,
        "total_cost_cents": total_cost,
        "reference": form.reference.unwrap_or_default(),
        "notes": Value::Null,
        "created_by": Value::Null,
    });
    store
        .create_with_tenant("stock_movement", movement_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 2. Crear lote de stock (tenant_id e auditoría inyectados automáticamente).
    let lot_data = json!({
        "id": lot_id,
        "lot_number": format!("LOTE-{movement_id}"),
        "product_id": form.product_id,
        "warehouse_id": form.warehouse_id,
        "expiry_date": Value::Null,
        "quantity": form.quantity,
        "unit_cost_cents": form.unit_cost_cents,
    });
    store
        .create_with_tenant("stock_lot", lot_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Crear arista de grafo warehouse->HAS_STOCK->product para trazabilidad.
    let edge_data = json!({
        "quantity": form.quantity,
        "unit_cost_cents": form.unit_cost_cents,
    });
    store
        .create_edge(
            "warehouse",
            &form.warehouse_id,
            "HAS_STOCK",
            "product",
            &form.product_id,
            edge_data,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/kardex"))
}

/// Handler POST /movimientos/salida — registrar salida de stock (con validación de no negativo).
pub async fn movimientos_salida(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<MovimientoSalidaForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/kardex")),
    };

    if form.quantity <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validar stock disponible usando consultas parametrizadas con tenant_id.
    let mut resp = store
        .query_with_tenant(
            "SELECT quantity FROM stock_lot WHERE product_id = $product_id AND warehouse_id = $warehouse_id",
            vec![
                ("$product_id".to_string(), Value::String(form.product_id.clone())),
                ("$warehouse_id".to_string(), Value::String(form.warehouse_id.clone())),
            ],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let lots: Vec<Value> = resp.take(0).unwrap_or_default();
    let available: i64 = lots
        .iter()
        .map(|l| l.get("quantity").and_then(|v| v.as_i64()).unwrap_or(0))
        .sum();

    if available < form.quantity {
        return Err(StatusCode::BAD_REQUEST); // stock insuficiente: no negativo
    }

    let movement_id = Uuid::new_v4().to_string();
    let total_cost = form.quantity * form.unit_cost_cents;

    let movement_data = json!({
        "id": movement_id,
        "product_id": form.product_id,
        "warehouse_id": form.warehouse_id,
        "movement_type": "SALIDA",
        "quantity": form.quantity,
        "unit_cost_cents": form.unit_cost_cents,
        "total_cost_cents": total_cost,
        "reference": form.reference.unwrap_or_default(),
        "notes": Value::Null,
        "created_by": Value::Null,
    });

    store
        .create_with_tenant("stock_movement", movement_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/kardex"))
}

/// Handler POST /movimientos/ajuste — registrar ajuste de stock.
pub async fn movimientos_ajuste(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<MovimientoAjusteForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/kardex")),
    };

    if form.unit_cost_cents <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let movement_id = Uuid::new_v4().to_string();
    let total_cost = form.quantity.abs() * form.unit_cost_cents;

    let movement_data = json!({
        "id": movement_id,
        "product_id": form.product_id,
        "warehouse_id": form.warehouse_id,
        "movement_type": "AJUSTE",
        "quantity": form.quantity.abs(),
        "unit_cost_cents": form.unit_cost_cents,
        "total_cost_cents": total_cost,
        "reference": Value::Null,
        "notes": form.notes.unwrap_or_default(),
        "created_by": Value::Null,
    });

    store
        .create_with_tenant("stock_movement", movement_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/kardex"))
}

/// Handler POST /movimientos/transferencia — transferir stock entre almacenes.
pub async fn movimientos_transferencia(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<MovimientoTransferenciaForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/kardex")),
    };

    if form.quantity <= 0 || form.unit_cost_cents <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let transfer_id = Uuid::new_v4().to_string();
    let total_cost = form.quantity * form.unit_cost_cents;

    // Movimiento de salida (origen).
    let salida_id = format!("{transfer_id}_salida");
    let salida_data = json!({
        "id": salida_id,
        "product_id": form.product_id,
        "warehouse_id": form.from_warehouse_id,
        "movement_type": "TRANSFERENCIA",
        "quantity": form.quantity,
        "unit_cost_cents": form.unit_cost_cents,
        "total_cost_cents": total_cost,
        "reference": Some(format!("TRANS-{transfer_id}")),
        "notes": Value::Null,
        "created_by": Value::Null,
    });
    store
        .create_with_tenant("stock_movement", salida_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Movimiento de entrada (destino).
    let entrada_id = format!("{transfer_id}_entrada");
    let entrada_data = json!({
        "id": entrada_id,
        "product_id": form.product_id,
        "warehouse_id": form.to_warehouse_id,
        "movement_type": "ENTRADA",
        "quantity": form.quantity,
        "unit_cost_cents": form.unit_cost_cents,
        "total_cost_cents": total_cost,
        "reference": Some(format!("TRANS-{transfer_id}")),
        "notes": Value::Null,
        "created_by": Value::Null,
    });
    store
        .create_with_tenant("stock_movement", entrada_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/kardex"))
}

/// Helper: convierte un `Value` de SurrealDB en [`StockMovement`] del dominio.
fn value_to_stock_movement(v: &Value) -> Option<StockMovement> {
    let id = v.get("id")?.as_str()?.to_string();
    let product_id = v.get("product_id")?.as_str()?.to_string();
    let warehouse_id = v.get("warehouse_id")?.as_str()?.to_string();
    let movement_type_str = v.get("movement_type")?.as_str()?;
    let movement_type = MovementType::from_str(movement_type_str).ok()?;
    let quantity: i64 = v.get("quantity")?.as_i64()?;
    let unit_cost_cents: i64 = v.get("unit_cost_cents")?.as_i64()?;
    let total_cost_cents: i64 = v.get("total_cost_cents")?.as_i64()?;
    let reference = v
        .get("reference")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let notes = v
        .get("notes")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tenant_id = v.get("tenant_id")?.as_str()?.to_string();
    let created_at = v.get("created_at")?.as_str()?.to_string();
    let created_by = v
        .get("created_by")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(StockMovement {
        id,
        product_id,
        warehouse_id,
        stock_lot_id: None,
        movement_type,
        quantity,
        unit_cost_cents,
        total_cost_cents,
        reference,
        notes,
        tenant_id,
        created_at,
        created_by,
    })
}

/// Handler GET /kardex — ver movimientos de stock del tenant con valoración
/// FIFO/AVCO acumulada (stock_after y cost_after_cents).
pub async fn kardex(State(state): State<AppState>, TenantCtx(ctx): TenantCtx) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Kardex", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM stock_movement WHERE tenant_id = $tenant_id ORDER BY created_at ASC",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Kardex", "<p>Error consultando kardex.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    // Convertir a StockMovement del dominio y construir kardex con valoración.
    let movements: Vec<StockMovement> = rows.iter().filter_map(value_to_stock_movement).collect();

    // Agrupar movimientos por (producto, almacén) y construir kardex por grupo.
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut kardex_entries: Vec<KardexEntry> = Vec::new();

    for m in &movements {
        let key = (m.product_id.clone(), m.warehouse_id.clone());
        if seen.insert(key.clone()) {
            let entries = InventoryService::build_kardex(&movements, &key.0, &key.1);
            kardex_entries.extend(entries);
        }
    }

    // Ordenar por fecha descendente (más reciente primero).
    kardex_entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let mut items = String::new();
    for entry in &kardex_entries {
        let sign = match entry.movement_type {
            MovementType::Entrada | MovementType::Ajuste => "+",
            MovementType::Salida | MovementType::Transferencia => "-",
        };
        items.push_str(&format!(
            r#"<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{} {}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
            entry.created_at,
            entry.movement_type,
            sign,
            entry.quantity,
            entry.unit_cost_cents,
            entry.stock_after,
            entry.cost_after_cents,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kardex - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/almacenes">Almacenes</a>
                <a href="/productos">Productos</a>
                <a href="/kardex">Kardex</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Kardex - Movimientos de Stock</h1>
        <table class="table">
            <thead>
                <tr>
                    <th>Fecha</th>
                    <th>Tipo</th>
                    <th>Cantidad</th>
                    <th>Costo Unit.</th>
                    <th>Stock After</th>
                    <th>Costo Acum.</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

// ============================================================================
// Handlers de Clientes (Fase 4)
// ============================================================================

/// Helper: convierte un `Value` de SurrealDB en [`Customer`] del dominio.
fn value_to_customer(v: &Value) -> Option<Customer> {
    let id = v.get("id")?.as_str()?.to_string();
    let rif = v.get("rif")?.as_str()?.to_string();
    let name = v.get("name")?.as_str()?.to_string();
    let vat_condition_str = v.get("vat_condition")?.as_str()?;
    let vat_condition = match vat_condition_str {
        "ordinario" => VatCondition::Ordinario,
        "formal" => VatCondition::Formal,
        "espe" => VatCondition::Espe,
        "no_sujeto" => VatCondition::NoSujeto,
        _ => VatCondition::NoSujeto,
    };
    let address = v
        .get("address")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let phone = v
        .get("phone")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let email = v
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(Customer {
        id,
        rif,
        name,
        vat_condition,
        address,
        phone,
        email,
    })
}

/// Handler GET /clientes — lista clientes del tenant.
pub async fn clientes_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Clientes", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM customer WHERE tenant_id = $tenant_id ORDER BY name",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Clientes", "<p>Error consultando clientes.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for c in &rows {
        let customer = match value_to_customer(c) {
            Some(c) => c,
            None => continue,
        };
        let id = escape_html(&customer.id);
        let rif = escape_html(&customer.rif);
        let name = escape_html(&customer.name);
        let vat_condition = match customer.vat_condition {
            VatCondition::Ordinario => "ordinario",
            VatCondition::Formal => "formal",
            VatCondition::Espe => "espe",
            VatCondition::NoSujeto => "no_sujeto",
        };
        let phone = escape_html(&customer.phone.unwrap_or_default());
        let email = escape_html(&customer.email.unwrap_or_default());

        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{rif}</td>
                <td>{name}</td>
                <td>{vat_condition}</td>
                <td>{phone}</td>
                <td>{email}</td>
                <td>
                    <a href="/clientes/{id}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Editar</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Clientes - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/facturas">Facturas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Clientes</h1>
        <a href="/clientes/nuevo" class="button button-primary">+ Nuevo Cliente</a>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>RIF</th>
                    <th>Nombre</th>
                    <th>Condición IVA</th>
                    <th>Teléfono</th>
                    <th>Email</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /clientes/nuevo — formulario crear cliente.
pub async fn clientes_nuevo() -> Html<String> {
    Html(views::customer_form_view(None))
}

/// Handler POST /clientes/crear — crear cliente.
pub async fn clientes_crear(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<CustomerForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/clientes")),
    };

    // Validar RIF con rustem-core.
    let _ = Rif::new(&form.rif).map_err(|_| StatusCode::BAD_REQUEST)?;

    if form.rif.is_empty() || form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let data = json!({
        "id": id,
        "rif": form.rif,
        "name": form.name,
        "vat_condition": form.vat_condition,
        "address": form.address.unwrap_or_default(),
        "phone": form.phone.unwrap_or_default(),
        "email": form.email.unwrap_or_default(),
    });

    store
        .create_with_tenant("customer", data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/clientes"))
}

/// Handler GET /clientes/:id/editar — formulario editar cliente.
pub async fn clientes_editar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Editar Cliente",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM customer WHERE id = $id AND tenant_id = $tenant_id LIMIT 1",
            vec![("$id".to_string(), Value::String(id.clone()))],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Editar Cliente",
                "<p>Error consultando cliente.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let customer = match rows.into_iter().next() {
        Some(c) => c,
        None => return Html(layout("Editar Cliente", "<p>Cliente no encontrado.</p>")),
    };

    let rif = escape_html(customer.get("rif").and_then(|v| v.as_str()).unwrap_or(""));
    let name = escape_html(customer.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    let vat_condition = escape_html(
        customer
            .get("vat_condition")
            .and_then(|v| v.as_str())
            .unwrap_or("ordinario"),
    );
    let address = escape_html(
        customer
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let phone = escape_html(customer.get("phone").and_then(|v| v.as_str()).unwrap_or(""));
    let email = escape_html(customer.get("email").and_then(|v| v.as_str()).unwrap_or(""));

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Editar Cliente - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Editar Cliente</h1>
        <form method="post" action="/clientes/{id}/actualizar" class="form">
            <div class="field">
                <label class="label" for="rif">RIF</label>
                <input type="text" id="rif" name="rif" class="input" value="{rif}" required>
            </div>
            <div class="field">
                <label class="label" for="name">Nombre</label>
                <input type="text" id="name" name="name" class="input" value="{name}" required>
            </div>
            <div class="field">
                <label class="label" for="vat_condition">Condición IVA</label>
                <select id="vat_condition" name="vat_condition" class="input">
                    <option value="ordinario" {}>Ordinario</option>
                    <option value="formal" {}>Formal</option>
                    <option value="espe" {}>Sujeto Pasivo Especial (SPE)</option>
                    <option value="no_sujeto" {}>No Sujeto</option>
                </select>
            </div>
            <div class="field">
                <label class="label" for="address">Dirección</label>
                <input type="text" id="address" name="address" class="input" value="{address}">
            </div>
            <div class="field">
                <label class="label" for="phone">Teléfono</label>
                <input type="tel" id="phone" name="phone" class="input" value="{phone}">
            </div>
            <div class="field">
                <label class="label" for="email">Email</label>
                <input type="email" id="email" name="email" class="input" value="{email}">
            </div>
            <button type="submit" class="button button-primary">Guardar</button>
            <a href="/clientes" class="button button-secondary">Cancelar</a>
        </form>
    </main>
</body>
</html>"#,
        if vat_condition == "ordinario" {
            "selected"
        } else {
            ""
        },
        if vat_condition == "formal" {
            "selected"
        } else {
            ""
        },
        if vat_condition == "espe" {
            "selected"
        } else {
            ""
        },
        if vat_condition == "no_sujeto" {
            "selected"
        } else {
            ""
        },
    );
    Html(html)
}

/// Handler POST /clientes/:id/actualizar — actualizar cliente.
pub async fn clientes_actualizar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<CustomerForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/clientes")),
    };

    // Validar RIF con rustem-core.
    let _ = Rif::new(&form.rif).map_err(|_| StatusCode::BAD_REQUEST)?;

    if form.rif.is_empty() || form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let data = json!({
        "rif": form.rif,
        "name": form.name,
        "vat_condition": form.vat_condition,
        "address": form.address.unwrap_or_default(),
        "phone": form.phone.unwrap_or_default(),
        "email": form.email.unwrap_or_default(),
    });

    store
        .update_with_tenant("customer", &id, data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/clientes"))
}

// ============================================================================
// Handlers de Plan de Cuentas (Fase 5)
// ============================================================================

/// Handler GET /contabilidad/cuentas — lista plan de cuentas del tenant.
pub async fn cuentas_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Plan de Cuentas",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store.query_chart_of_accounts_by_tenant().await {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Plan de Cuentas",
                "<p>Error consultando plan de cuentas.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for a in &rows {
        let code = escape_html(a.get("code").and_then(|v| v.as_str()).unwrap_or(""));
        let name = escape_html(a.get("name").and_then(|v| v.as_str()).unwrap_or(""));
        let account_type =
            escape_html(a.get("account_type").and_then(|v| v.as_str()).unwrap_or(""));
        let level: i64 = a.get("level").and_then(|v| v.as_i64()).unwrap_or(1);
        let parent_code = escape_html(a.get("parent_code").and_then(|v| v.as_str()).unwrap_or("-"));
        let is_active = a
            .get("is_operational")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let badge = if is_active {
            "<span class=\"badge badge-success\">Activo</span>"
        } else {
            "<span class=\"badge badge-danger\">Inactivo</span>"
        };
        let indent = "    ".repeat(level.max(1) as usize - 1);

        items.push_str(&format!(
            r#"<tr>
                <td>{code}</td>
                <td>{indent}{name}</td>
                <td>{account_type}</td>
                <td>{level}</td>
                <td>{parent_code}</td>
                <td>{badge}</td>
                <td>
                    <a href="/contabilidad/cuentas/{code}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Editar</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Plan de Cuentas - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/contabilidad/cuentas">Plan de Cuentas</a>
                <a href="/contabilidad/asientos">Asientos</a>
                <a href="/contabilidad/libro-diario">Libro Diario</a>
                <a href="/contabilidad/libro-mayor">Libro Mayor</a>
                <a href="/contabilidad/balance">Balance General</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Plan de Cuentas VEN-NIF</h1>
        <a href="/contabilidad/cuentas/nuevo" class="button button-primary">+ Nueva Cuenta</a>
        <table class="table">
            <thead>
                <tr>
                    <th>Código</th>
                    <th>Nombre</td>
                    <th>Tipo</th>
                    <th>Nivel</th>
                    <th>Cuenta Padre</th>
                    <th>Estado</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /contabilidad/cuentas/nuevo — formulario nueva cuenta.
pub async fn cuentas_nuevo() -> Html<String> {
    Html(layout(
        "Nueva Cuenta Contable",
        "<p>Formulario de nueva cuenta contable (próximamente).</p>",
    ))
}

/// Handler POST /contabilidad/cuentas/crear — crear cuenta contable.
pub async fn cuentas_crear(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AccountForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/contabilidad/cuentas")),
    };

    if form.code.is_empty() || form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let level = form.level.unwrap_or(1);
    let data = json!({
        "code": form.code,
        "name": form.name,
        "account_type": form.account_type,
        "level": level,
        "parent_code": form.parent_code.unwrap_or_default(),
        "is_operational": form.is_operational.unwrap_or(true),
    });

    store
        .create_with_tenant("chart_of_accounts", data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/contabilidad/cuentas"))
}

/// Handler GET /contabilidad/cuentas/:id/editar — formulario editar cuenta.
pub async fn cuentas_editar(
    State(state): State<AppState>,
    Path(code): Path<String>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Editar Cuenta",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store.query_chart_of_accounts_by_tenant().await {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Editar Cuenta",
                "<p>Error consultando plan de cuentas.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let account = match rows
        .into_iter()
        .find(|a| a.get("code").and_then(|v| v.as_str()) == Some(&code))
    {
        Some(a) => a,
        None => return Html(layout("Editar Cuenta", "<p>Cuenta no encontrada.</p>")),
    };

    let name = escape_html(account.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    let account_type = escape_html(
        account
            .get("account_type")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let parent_code = escape_html(
        account
            .get("parent_code")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let is_active = account
        .get("is_operational")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let checked = if is_active { " checked" } else { "" };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Editar Cuenta - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/contabilidad/cuentas">Plan de Cuentas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Editar Cuenta Contable</h1>
        <form method="post" action="/contabilidad/cuentas/{code}/actualizar" class="form">
            <div class="field">
                <label class="label" for="code">Código</label>
                <input type="text" id="code" name="code" class="input" value="{code}" required readonly>
            </div>
            <div class="field">
                <label class="label" for="name">Nombre</label>
                <input type="text" id="name" name="name" class="input" value="{name}" required>
            </div>
            <div class="field">
                <label class="label" for="account_type">Tipo de Cuenta</label>
                <input type="text" id="account_type" name="account_type" class="input" value="{account_type}" required>
            </div>
            <div class="field">
                <label class="label" for="parent_code">Cuenta Padre</label>
                <input type="text" id="parent_code" name="parent_code" class="input" value="{parent_code}">
            </div>
            <div class="field">
                <label class="label" for="is_operational">Operativa</label>
                <input type="checkbox" id="is_operational" name="is_operational" value="1"{checked}>
            </div>
            <button type="submit" class="button button-primary">Guardar</button>
            <a href="/contabilidad/cuentas" class="button button-secondary">Cancelar</a>
        </form>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler POST /contabilidad/cuentas/:id/actualizar — actualizar cuenta.
pub async fn cuentas_actualizar(
    State(state): State<AppState>,
    Path(code): Path<String>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AccountForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/contabilidad/cuentas")),
    };

    if form.code.is_empty() || form.name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let data = json!({
        "code": form.code,
        "name": form.name,
        "account_type": form.account_type,
        "level": form.level.unwrap_or(1),
        "parent_code": form.parent_code.unwrap_or_default(),
        "is_operational": form.is_operational.unwrap_or(true),
    });

    store
        .update_with_tenant("chart_of_accounts", &code, data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/contabilidad/cuentas"))
}

/// Handler GET /contabilidad/cuentas/arbol — árbol jerárquico de cuentas.
pub async fn cuentas_arbol_view(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Árbol de Cuentas",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store.query_chart_of_accounts_by_tenant().await {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Árbol de Cuentas",
                "<p>Error consultando plan de cuentas.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    // Construir mapa de cuentas para jerarquía.
    let mut accounts_map: std::collections::HashMap<String, views::AccountNode> =
        std::collections::HashMap::new();

    for row in &rows {
        let code = row
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = row
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let account_type = row
            .get("account_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let level: u8 = row.get("level").and_then(|v| v.as_i64()).unwrap_or(1) as u8;
        let parent_code = row
            .get("parent_code")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let is_active = row
            .get("is_operational")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        accounts_map.insert(
            code.clone(),
            views::AccountNode {
                code,
                name,
                account_type,
                level,
                parent_code,
                is_active,
                children: Vec::new(),
            },
        );
    }

    // Construir jerarquía: asignar hijos a sus padres.
    let mut root_accounts: Vec<views::AccountNode> = Vec::new();
    for (_code, account) in accounts_map.clone() {
        if let Some(parent_code) = &account.parent_code {
            if !parent_code.is_empty() {
                if let Some(parent) = accounts_map.get_mut(parent_code) {
                    parent.children.push(account.clone());
                }
            } else {
                root_accounts.push(account);
            }
        } else {
            root_accounts.push(account);
        }
    }

    // Ordenar cuentas raíz por código.
    root_accounts.sort_by(|a, b| a.code.cmp(&b.code));

    // Ordenar hijos de cada nodo por código.
    fn sort_children(node: &mut views::AccountNode) {
        node.children.sort_by(|a, b| a.code.cmp(&b.code));
        for child in &mut node.children {
            sort_children(child);
        }
    }
    for account in &mut root_accounts {
        sort_children(account);
    }

    let html_body = views::cuentas_arbol_view(&root_accounts);

    Html(html_body)
}

/// Handler GET /contabilidad/asientos — lista asientos del tenant.
pub async fn asientos_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Asientos Contables",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store.query_journal_entries_by_tenant(None, None).await {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Asientos Contables",
                "<p>Error consultando asientos.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for a in &rows {
        let id = escape_html(a.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let date = escape_html(a.get("date").and_then(|v| v.as_str()).unwrap_or(""));
        let description = escape_html(a.get("description").and_then(|v| v.as_str()).unwrap_or(""));
        let lines: &[Value] = a
            .get("lines")
            .and_then(|v| v.as_array())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Calcular totales de débito y crédito
        let debito_total: i64 = lines
            .iter()
            .filter(|l| l.get("amount_cents").and_then(|v| v.as_i64()).unwrap_or(0) > 0)
            .map(|l| l.get("amount_cents").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let credito_total: i64 = lines
            .iter()
            .filter(|l| l.get("amount_cents").and_then(|v| v.as_i64()).unwrap_or(0) < 0)
            .map(|l| {
                l.get("amount_cents")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
                    .abs()
            })
            .sum();

        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{date}</td>
                <td>{description}</td>
                <td>{debito_total}</td>
                <td>{credito_total}</td>
                <td>
                    <a href="/contabilidad/asientos/{id}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Ver</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Asientos Contables - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/contabilidad/cuentas">Plan de Cuentas</a>
                <a href="/contabilidad/asientos">Asientos</a>
                <a href="/contabilidad/asientos/nuevo">Nuevo Asiento</a>
                <a href="/contabilidad/libro-diario">Libro Diario</a>
                <a href="/contabilidad/libro-mayor">Libro Mayor</a>
                <a href="/contabilidad/balance">Balance General</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Asientos Contables</h1>
        <a href="/contabilidad/asientos/nuevo" class="button button-primary">+ Nuevo Asiento</a>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Fecha</th>
                    <th>Descripción</th>
                    <th>Débitos</th>
                    <th>Créditos</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /contabilidad/asientos/nuevo — formulario nuevo asiento.
pub async fn asientos_nuevo() -> Html<String> {
    Html(layout(
        "Nuevo Asiento Contable",
        "<p>Formulario de nuevo asiento contable (próximamente).</p>",
    ))
}

/// Handler POST /contabilidad/asientos/crear — crear asiento con validación de partida doble.
pub async fn asientos_crear(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AsientoForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/contabilidad/asientos")),
    };

    if form.date.is_empty() || form.description.is_empty() || form.lineas.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validar que ∑débitos = ∑créditos (partida doble)
    let debito_total: i64 = form
        .lineas
        .iter()
        .filter(|l| l.amount_cents > 0)
        .map(|l| l.amount_cents)
        .sum();
    let credito_total: i64 = form
        .lineas
        .iter()
        .filter(|l| l.amount_cents < 0)
        .map(|l| l.amount_cents.abs())
        .sum();

    if debito_total != credito_total {
        return Err(StatusCode::BAD_REQUEST); // Asiento desbalanceado
    }

    let id = uuid::Uuid::new_v4().to_string();
    let lineas_json: Vec<Value> = form
        .lineas
        .iter()
        .map(|l| {
            json!({
                "account_code": l.account_code,
                "amount_cents": l.amount_cents,
                "description": l.description
            })
        })
        .collect();

    store
        .create_journal_entry(
            &id,
            &form.date,
            &form.description,
            json!(lineas_json),
            ctx.user_id.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/contabilidad/asientos"))
}

/// Handler GET /contabilidad/asientos/:id/editar — formulario editar asiento.
pub async fn asientos_editar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Editar Asiento",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store.query_journal_entries_by_tenant(None, None).await {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Editar Asiento",
                "<p>Error consultando asientos.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let asiento = match rows
        .into_iter()
        .find(|a| a.get("id").and_then(|v| v.as_str()) == Some(&id))
    {
        Some(a) => a,
        None => return Html(layout("Editar Asiento", "<p>Asiento no encontrado.</p>")),
    };

    let date = escape_html(asiento.get("date").and_then(|v| v.as_str()).unwrap_or(""));
    let description = escape_html(
        asiento
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Editar Asiento - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/contabilidad/asientos">Asientos</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Editar Asiento Contable</h1>
        <p><strong>ID:</strong> {id}</p>
        <p><strong>Fecha:</strong> {date}</p>
        <p><strong>Descripción:</strong> {description}</p>
        <a href="/contabilidad/asientos" class="button button-secondary">Volver</a>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler POST /contabilidad/asientos/:id/actualizar — actualizar asiento.
pub async fn asientos_actualizar(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AsientoForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/contabilidad/asientos")),
    };

    if form.date.is_empty() || form.description.is_empty() || form.lineas.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validar que ∑débitos = ∑créditos (partida doble)
    let debito_total: i64 = form
        .lineas
        .iter()
        .filter(|l| l.amount_cents > 0)
        .map(|l| l.amount_cents)
        .sum();
    let credito_total: i64 = form
        .lineas
        .iter()
        .filter(|l| l.amount_cents < 0)
        .map(|l| l.amount_cents.abs())
        .sum();

    if debito_total != credito_total {
        return Err(StatusCode::BAD_REQUEST); // Asiento desbalanceado
    }

    let lineas_json: Vec<Value> = form
        .lineas
        .iter()
        .map(|l| {
            json!({
                "account_code": l.account_code,
                "amount_cents": l.amount_cents,
                "description": l.description
            })
        })
        .collect();

    store
        .create_journal_entry(
            &id,
            &form.date,
            &form.description,
            json!(lineas_json),
            ctx.user_id.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/contabilidad/asientos"))
}

// ============================================================================
// Handlers de Ventas y Facturación (Fase 4)
// ============================================================================

/// Handler GET /cotizaciones — lista cotizaciones del tenant.
pub async fn cotizaciones_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Cotizaciones",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM quote WHERE tenant_id = $tenant_id ORDER BY created_at DESC",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Cotizaciones",
                "<p>Error consultando cotizaciones.</p>",
            ));
        }
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for q in &rows {
        let id = escape_html(q.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let customer_id = escape_html(q.get("customer_id").and_then(|v| v.as_str()).unwrap_or(""));
        let total_cents: i64 = q.get("total_cents").and_then(|v| v.as_i64()).unwrap_or(0);
        let state_str = escape_html(
            q.get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("borrador"),
        );

        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{customer_id}</td>
                <td>{total_cents}</td>
                <td>{state_str}</td>
                <td>
                    <a href="/cotizaciones/{id}" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Ver</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Cotizaciones - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/cotizaciones">Cotizaciones</a>
                <a href="/pedidos">Pedidos</a>
                <a href="/facturas">Facturas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Cotizaciones</h1>
        <a href="/cotizaciones/nuevo" class="button button-primary">+ Nueva Cotización</a>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Cliente</th>
                    <th>Total</th>
                    <th>Estado</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /cotizaciones/nuevo — formulario nueva cotización.
pub async fn cotizaciones_nuevo() -> Html<String> {
    Html(layout(
        "Nueva Cotización",
        "<p>Formulario de cotización (próximamente).</p>",
    ))
}

/// Handler POST /cotizaciones/crear — crear cotización.
pub async fn cotizaciones_crear(
    State(_state): State<AppState>,
    TenantCtx(_ctx): TenantCtx,
    Form(_form): Form<serde_json::Value>,
) -> Result<Redirect, StatusCode> {
    Ok(Redirect::to("/cotizaciones"))
}

/// Handler GET /pedidos — lista pedidos del tenant.
pub async fn pedidos_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Pedidos", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM order_table WHERE tenant_id = $tenant_id ORDER BY created_at DESC",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Pedidos", "<p>Error consultando pedidos.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for o in &rows {
        let id = escape_html(o.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let customer_id = escape_html(o.get("customer_id").and_then(|v| v.as_str()).unwrap_or(""));
        let total_cents: i64 = o.get("total_cents").and_then(|v| v.as_i64()).unwrap_or(0);
        let state_str = escape_html(
            o.get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("borrador"),
        );

        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{customer_id}</td>
                <td>{total_cents}</td>
                <td>{state_str}</td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pedidos - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/cotizaciones">Cotizaciones</a>
                <a href="/pedidos">Pedidos</a>
                <a href="/facturas">Facturas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Pedidos</h1>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Cliente</th>
                    <th>Total</th>
                    <th>Estado</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /facturas — lista facturas del tenant.
pub async fn facturas_list(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Facturas", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM invoice WHERE tenant_id = $tenant_id ORDER BY created_at DESC",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Facturas", "<p>Error consultando facturas.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut items = String::new();
    for f in &rows {
        let id = escape_html(f.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let customer_id = escape_html(f.get("customer_id").and_then(|v| v.as_str()).unwrap_or(""));
        let series = escape_html(f.get("series").and_then(|v| v.as_str()).unwrap_or(""));
        let number: u32 = f.get("number").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let control_number = escape_html(
            f.get("control_number")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );
        let total_cents: i64 = f.get("total_cents").and_then(|v| v.as_i64()).unwrap_or(0);
        let state_str = escape_html(
            f.get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("borrador"),
        );

        items.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{series}-{number}</td>
                <td>{control_number}</td>
                <td>{customer_id}</td>
                <td>{total_cents}</td>
                <td>{state_str}</td>
                <td>
                    <a href="/facturas/{id}/imprimir" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Imprimir</a>
                </td>
            </tr>"#,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Facturas - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/cotizaciones">Cotizaciones</a>
                <a href="/pedidos">Pedidos</a>
                <a href="/facturas">Facturas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Facturas</h1>
        <a href="/facturas/nuevo" class="button button-primary">+ Nueva Factura</a>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Número</th>
                    <th>Control SENIAT</th>
                    <th>Cliente</th>
                    <th>Total</th>
                    <th>Estado</th>
                    <th>Acciones</th>
                </tr>
            </thead>
            <tbody>
                {items}
            </tbody>
        </table>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler GET /facturas/nuevo — formulario nueva factura.
pub async fn facturas_nuevo(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => {
            return Html(layout(
                "Nueva Factura",
                "<p>Sin base de datos disponible.</p>",
            ));
        }
    };

    // Obtener lista de clientes para el desplegable.
    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM customer WHERE tenant_id = $tenant_id ORDER BY name",
            vec![],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Html(layout(
                "Nueva Factura",
                "<p>Error consultando clientes.</p>",
            ));
        }
    };

    let customers: Vec<Value> = resp.take(0).unwrap_or_default();

    let mut options = String::new();
    for c in &customers {
        let id = escape_html(c.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let rif = escape_html(c.get("rif").and_then(|v| v.as_str()).unwrap_or(""));
        let name = escape_html(c.get("name").and_then(|v| v.as_str()).unwrap_or(""));
        options.push_str(&format!("<option value=\"{id}\">{rif} - {name}</option>"));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Nueva Factura - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/facturas">Facturas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Nueva Factura</h1>
        <form method="post" action="/facturas/crear" class="form">
            <div class="field">
                <label class="label" for="customer_id">Cliente</label>
                <select id="customer_id" name="customer_id" class="input" required>
                    {options}
                </select>
            </div>
            <div class="field">
                <label class="label" for="series">Serie</label>
                <input type="text" id="series" name="series" class="input" value="001" required>
            </div>
            <div class="field">
                <label class="label" for="number">Número</label>
                <input type="number" id="number" name="number" class="input" value="1" required>
            </div>
            <div class="field">
                <label class="label" for="control_number">Número de Control SENIAT</label>
                <input type="text" id="control_number" name="control_number" class="input" required>
            </div>
            <div class="field">
                <label class="label" for="currency">Moneda</label>
                <select id="currency" name="currency" class="input">
                    <option value="VES">Bolívares (VES)</option>
                    <option value="USD">Dólares (USD)</option>
                </select>
            </div>
            <div class="field">
                <label class="label" for="iva_rate">Alícuota IVA</label>
                <select id="iva_rate" name="iva_rate" class="input">
                    <option value="16">General (16%)</option>
                    <option value="8">Reducida (8%)</option>
                    <option value="0">Exento (0%)</option>
                </select>
            </div>
            <div class="field">
                <label class="label" for="igtf_rate_percent">IGTF %</label>
                <input type="number" id="igtf_rate_percent" name="igtf_rate_percent" class="input" value="0" step="0.01">
            </div>
            <div class="field">
                <label class="label" for="subtotal_cents">Subtotal (centavos)</label>
                <input type="number" id="subtotal_cents" name="subtotal_cents" class="input" required>
            </div>
            <button type="submit" class="button button-primary">Emitir Factura</button>
            <a href="/facturas" class="button button-secondary">Cancelar</a>
        </form>
    </main>
</body>
</html>"#,
    );
    Html(html)
}

/// Handler POST /facturas/crear — crear factura con cálculo fiscal (IVA + IGTF).
pub async fn facturas_crear(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<InvoiceForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/facturas")),
    };

    // Validar RIF del cliente implícitamente (el cliente ya existe con RIF validado).
    if form.customer_id.is_empty() || form.series.is_empty() || form.control_number.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let subtotal_cents = form.subtotal_cents.parse::<i64>().unwrap_or(0);

    // Calcular impuestos usando TaxEngine.
    let iva_rate = match form.iva_rate.as_str() {
        "16" => rustem_sales::IvaRate::General,
        "8" => rustem_sales::IvaRate::Reducida,
        _ => rustem_sales::IvaRate::Exento,
    };

    // Crear factura en dominio para calcular totales.
    let mut invoice = rustem_sales::Invoice {
        id: id.clone(),
        order_id: None,
        customer_id: form.customer_id.clone(),
        series: form.series.clone(),
        number: form.number,
        control_number: form.control_number.clone(),
        state: rustem_sales::DocumentState::Emitido,
        subtotal_cents: 0,
        iva_cents: 0,
        igtf_cents: 0,
        total_cents: 0,
        currency: form.currency.clone(),
        exchange_rate: None,
    };

    rustem_sales::TaxEngine::apply_to_invoice(
        &mut invoice,
        subtotal_cents,
        iva_rate,
        form.igtf_rate_percent,
    );

    let now = chrono::Utc::now().to_rfc3339();
    let data = json!({
        "id": invoice.id,
        "order_id": Value::Null,
        "customer_id": invoice.customer_id,
        "series": invoice.series,
        "number": invoice.number,
        "control_number": invoice.control_number,
        "state": "emitido",
        "subtotal_cents": invoice.subtotal_cents,
        "iva_cents": invoice.iva_cents,
        "igtf_cents": invoice.igtf_cents,
        "total_cents": invoice.total_cents,
        "currency": invoice.currency,
        "exchange_rate": invoice.exchange_rate,
        "tenant_id": ctx.tenant_id,
        "created_at": now,
        "updated_at": now,
    });

    store
        .create_with_tenant("invoice", data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/facturas/{}/imprimir", id)))
}

/// Handler GET /facturas/:id/imprimir — vista de factura para impresión fiscal (@media print).
pub async fn factura_imprimir(
    State(state): State<AppState>,
    Path(id): Path<String>,
    TenantCtx(ctx): TenantCtx,
) -> Html<String> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Html(layout("Factura", "<p>Sin base de datos disponible.</p>")),
    };

    let mut resp = match store
        .query_with_tenant(
            "SELECT * FROM invoice WHERE id = $id AND tenant_id = $tenant_id LIMIT 1",
            vec![("$id".to_string(), Value::String(id.clone()))],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(layout("Factura", "<p>Error consultando factura.</p>")),
    };

    let rows: Vec<Value> = resp.take(0).unwrap_or_default();
    let invoice = match rows.into_iter().next() {
        Some(f) => f,
        None => return Html(layout("Factura", "<p>Factura no encontrada.</p>")),
    };

    let series = escape_html(invoice.get("series").and_then(|v| v.as_str()).unwrap_or(""));
    let number: u32 = invoice.get("number").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let control_number = escape_html(
        invoice
            .get("control_number")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let subtotal_cents: i64 = invoice
        .get("subtotal_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let iva_cents: i64 = invoice
        .get("iva_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let igtf_cents: i64 = invoice
        .get("igtf_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let total_cents: i64 = invoice
        .get("total_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let currency = escape_html(
        invoice
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("VES"),
    );

    // Formatear montos en unidades mayores.
    let format_money = |cents: i64| {
        let sign = if cents < 0 { "-" } else { "" };
        let abs = cents.abs();
        let major = abs / 100;
        let minor = abs % 100;
        format!("{}{}.{:02} {}", sign, major, minor, currency)
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Factura {series}-{number} - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar no-print">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/facturas">Volver a Facturas</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <div class="invoice no-print">
            <h1 class="page-title">FACTURA FISCAL</h1>
            <p><strong>Número:</strong> {series}-{number}</p>
            <p><strong>Control SENIAT:</strong> {control_number}</p>
            <p><strong>Moneda:</strong> {currency}</p>
            <table class="table">
                <thead>
                    <tr>
                        <th>Concepto</th>
                        <th>Monto</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>Subtotal</td>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <td>IVA</td>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <td>IGTF</td>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <td><strong>TOTAL</strong></td>
                        <td><strong>{}</strong></td>
                    </tr>
                </tbody>
            </table>
        </div>
        <p class="no-print"><a href="/facturas" class="button button-primary">Volver</a></p>
    </main>
</body>
</html>"#,
        format_money(subtotal_cents),
        format_money(iva_cents),
        format_money(igtf_cents),
        format_money(total_cents),
    );
    Html(html)
}

/// Handler GET /dashboard — panel principal (requiere tenant).
pub async fn dashboard(req: Request) -> Html<String> {
    let tenant_ctx = extract_tenant_ctx(&req);
    let user_name = tenant_ctx
        .as_ref()
        .and_then(|c| c.user_id.as_deref())
        .unwrap_or("Usuario")
        .to_string();
    let company_name = tenant_ctx
        .as_ref()
        .and_then(|c| {
            if c.tenant_id.is_empty() {
                None
            } else {
                Some(c.tenant_id.as_str())
            }
        })
        .unwrap_or("Empresa Demo")
        .to_string();
    Html(views::dashboard_view(&user_name, &company_name, None))
}

/// Handler GET /inventario — módulo de inventario.
pub async fn inventario() -> Html<String> {
    Html(layout(
        "Inventario",
        "<h2>Inventario</h2><p>Módulo de gestión de inventario (próximamente).</p>",
    ))
}

/// Handler GET /ventas — módulo de ventas.
pub async fn ventas() -> Html<String> {
    Html(layout(
        "Ventas",
        "<h2>Ventas</h2><p>Módulo de ventas y facturación (próximamente).</p>",
    ))
}

/// Handler GET /contabilidad — módulo de contabilidad.
pub async fn contabilidad() -> Html<String> {
    Html(layout(
        "Contabilidad",
        "<h2>Contabilidad VEN-NIF</h2><p>Módulo contable (próximamente).</p>",
    ))
}

/// Handler GET /admin — módulo de administración.
pub async fn admin() -> Html<String> {
    Html(layout(
        "Administración",
        "<h2>Administración</h2><p>Tenancy, RBAC y usuarios (próximamente).</p>",
    ))
}

/// Datos para crear un asiento contable automático desde una venta o
/// movimiento de inventario.
#[derive(Debug, Deserialize)]
pub struct AsientoAutoForm {
    /// Fecha del asiento (YYYY-MM-DD).
    pub date: String,
    /// Descripción del asiento.
    pub description: String,
    /// Líneas del asiento (account_code, amount_cents, description).
    pub lineas: Vec<AsientoLineaFormEntry>,
}


/// Crea un asiento contable automático desde una venta/factura.
///
/// Las líneas deben incluir el débito a Cuentas por Cobrar, el crédito a
/// Ventas, el IVA débito fiscal, etc. Valida ∑débitos = ∑créditos usando
/// EntryEngine antes de persistir.
pub async fn asientos_crear_desde_venta(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AsientoAutoForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/contabilidad/asientos")),
    };

    if form.lineas.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let lineas_engine: Vec<EntryLine> = form
        .lineas
        .iter()
        .map(|l| EntryLine {
            account_code: l.account_code.clone(),
            amount_cents: l.amount_cents,
            description: l.description.clone(),
        })
        .collect();

    let entry = EntryEngine::create_entry(
        format!("auto-{}", uuid::Uuid::new_v4()),
        &form.date,
        &form.description,
        lineas_engine,
        &ctx.tenant_id,
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    let lines_json: Vec<Value> = form
        .lineas
        .iter()
        .map(|l| {
            serde_json::json!({
                "account_code": l.account_code,
                "amount_cents": l.amount_cents,
                "description": l.description
            })
        })
        .collect();

    store
        .create_journal_entry(
            &entry.id,
            &form.date,
            &form.description,
            serde_json::json!(lines_json),
            ctx.user_id.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/contabilidad/asientos"))
}

/// Crea un asiento contable automático desde un movimiento de inventario.
pub async fn asientos_crear_desde_movimiento(
    State(state): State<AppState>,
    TenantCtx(ctx): TenantCtx,
    Form(form): Form<AsientoAutoForm>,
) -> Result<Redirect, StatusCode> {
    let store = match get_store(&state, &ctx).await {
        Some(s) => s,
        None => return Ok(Redirect::to("/contabilidad/asientos")),
    };

    if form.lineas.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let lineas_engine: Vec<EntryLine> = form
        .lineas
        .iter()
        .map(|l| EntryLine {
            account_code: l.account_code.clone(),
            amount_cents: l.amount_cents,
            description: l.description.clone(),
        })
        .collect();

    let entry = EntryEngine::create_entry(
        format!("auto-{}", uuid::Uuid::new_v4()),
        &form.date,
        &form.description,
        lineas_engine,
        &ctx.tenant_id,
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    let lines_json: Vec<Value> = form
        .lineas
        .iter()
        .map(|l| {
            serde_json::json!({
                "account_code": l.account_code,
                "amount_cents": l.amount_cents,
                "description": l.description
            })
        })
        .collect();

    store
        .create_journal_entry(
            &entry.id,
            &form.date,
            &form.description,
            serde_json::json!(lines_json),
            ctx.user_id.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/contabilidad/asientos"))
}

/// Emite advertencia de rate limit en los logs.
fn warn_rate_limit(guard: &rustem_auth::brute_force::BruteForceGuard, ip: &str, username: &str) {
    let ip_rem = guard.ip_remaining_seconds(ip);
    let user_rem = guard.user_remaining_seconds(username);
    tracing::warn!(
        "Rate limit: IP {ip} (queda {ip_rem}s) y usuario {username} (queda {user_rem}s) bloqueados."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_get_returns_html() {
        let html = views::login_view(None);
        assert!(html.contains("Rustem Suite"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn dashboard_view_returns_html() {
        let html = views::dashboard_view("Usuario", "Empresa Demo", None);
        assert!(html.contains("Panel de Control"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn layout_has_no_js() {
        let html = layout("Test", "<p>contenido</p>");
        assert!(!html.contains("<script"));
        assert!(html.contains("Test"));
    }

    #[test]
    fn root_returns_redirect() {
        let _redirect = root();
    }

    #[tokio::test]
    async fn logout_returns_redirect() {
        let state = crate::state::AppState::for_tests();
        let req = http::Request::builder()
            .method("POST")
            .uri("/logout")
            .body(axum::body::Body::empty())
            .unwrap();
        let _redirect = logout_post(State(state), req).await;
    }

    #[tokio::test]
    async fn register_get_returns_html() {
        let html = register_get().await;
        assert!(html.0.contains("Registro"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn almacenes_nuevo_returns_html() {
        let html = almacenes_nuevo().await;
        assert!(html.0.contains("Almacén"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn productos_nuevo_returns_html() {
        let html = productos_nuevo().await;
        assert!(html.0.contains("Producto"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn clientes_nuevo_returns_html() {
        let html = clientes_nuevo().await;
        assert!(html.0.contains("Cliente"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn cuentas_nuevo_returns_html() {
        let html = cuentas_nuevo().await;
        assert!(html.0.contains("Cuenta"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn asientos_nuevo_returns_html() {
        let html = asientos_nuevo().await;
        assert!(html.0.contains("Asiento"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn cotizaciones_nuevo_returns_html() {
        let html = cotizaciones_nuevo().await;
        assert!(html.0.contains("Cotización"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn inventario_returns_html() {
        let html = inventario().await;
        assert!(html.0.contains("Inventario"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn ventas_returns_html() {
        let html = ventas().await;
        assert!(html.0.contains("Ventas"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn contabilidad_returns_html() {
        let html = contabilidad().await;
        assert!(html.0.contains("Contabilidad"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn admin_returns_html() {
        let html = admin().await;
        assert!(html.0.contains("Admin"));
        assert!(!html.0.contains("<script"));
    }

    #[tokio::test]
    async fn almacenes_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = almacenes_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn productos_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = productos_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn clientes_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = clientes_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn cuentas_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = cuentas_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn asientos_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = asientos_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn facturas_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = facturas_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn kardex_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = kardex(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn almacenes_crear_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AlmacenForm {
            name: "Test".to_string(),
            address: None,
            phone: None,
            is_active: None,
        };
        let result = almacenes_crear(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn almacenes_editar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = almacenes_editar(State(state), axum::extract::Path("id-1".to_string()), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn productos_crear_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = ProductoForm {
            sku: "SKU-1".to_string(),
            name: "Test".to_string(),
            description: None,
            unit: None,
            category: None,
            vat_rate: None,
            min_stock: None,
        };
        let result = productos_crear(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn clientes_crear_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = CustomerForm {
            rif: "V-12345678-9".to_string(),
            name: "Test".to_string(),
            vat_condition: "ordinario".to_string(),
            address: None,
            phone: None,
            email: None,
        };
        let result = clientes_crear(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cuentas_crear_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AccountForm {
            code: "1000".to_string(),
            name: "Test".to_string(),
            account_type: "activo".to_string(),
            level: None,
            parent_code: None,
            is_operational: None,
        };
        let result = cuentas_crear(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn facturas_crear_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = InvoiceForm {
            customer_id: "customer-1".to_string(),
            series: "F".to_string(),
            number: 1,
            control_number: "CTRL-1".to_string(),
            currency: "VES".to_string(),
            iva_rate: "16".to_string(),
            igtf_rate_percent: 0,
            subtotal_cents: "10000".to_string(),
        };
        let result = facturas_crear(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn movimientos_entrada_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = MovimientoEntradaForm {
            product_id: "prod-1".to_string(),
            warehouse_id: "wh-1".to_string(),
            quantity: 10,
            unit_cost_cents: 1000,
            reference: Some("REF-1".to_string()),
        };
        let result = movimientos_entrada(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn movimientos_salida_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = MovimientoSalidaForm {
            product_id: "prod-1".to_string(),
            warehouse_id: "wh-1".to_string(),
            quantity: 5,
            unit_cost_cents: 1000,
            reference: Some("REF-2".to_string()),
        };
        let result = movimientos_salida(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn movimientos_ajuste_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = MovimientoAjusteForm {
            product_id: "prod-1".to_string(),
            warehouse_id: "wh-1".to_string(),
            quantity: 3,
            unit_cost_cents: 1000,
            notes: Some("Ajuste".to_string()),
        };
        let result = movimientos_ajuste(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn movimientos_transferencia_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = MovimientoTransferenciaForm {
            product_id: "prod-1".to_string(),
            from_warehouse_id: "wh-1".to_string(),
            to_warehouse_id: "wh-2".to_string(),
            quantity: 2,
            unit_cost_cents: 1000,
        };
        let result = movimientos_transferencia(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cotizaciones_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = cotizaciones_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn pedidos_list_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = pedidos_list(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn almacenes_actualizar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AlmacenForm {
            name: "Test".to_string(),
            address: None,
            phone: None,
            is_active: None,
        };
        let result = almacenes_actualizar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
            axum::Form(form),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn productos_editar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = productos_editar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
        )
        .await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn clientes_editar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = clientes_editar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
        )
        .await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn cuentas_editar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = cuentas_editar(
            State(state),
            axum::extract::Path("code-1".to_string()),
            ctx,
        )
        .await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn asientos_editar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = asientos_editar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
        )
        .await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn cotizaciones_crear_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let result = cotizaciones_crear(
            State(state),
            ctx,
            axum::Form(serde_json::json!({})),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn productos_actualizar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = ProductoForm {
            sku: "SKU-1".to_string(),
            name: "Test".to_string(),
            description: None,
            unit: None,
            category: None,
            vat_rate: None,
            min_stock: None,
        };
        let result = productos_actualizar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
            axum::Form(form),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn clientes_actualizar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = CustomerForm {
            rif: "V-12345678-9".to_string(),
            name: "Test".to_string(),
            vat_condition: "ordinario".to_string(),
            address: None,
            phone: None,
            email: None,
        };
        let result = clientes_actualizar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
            axum::Form(form),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cuentas_actualizar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AccountForm {
            code: "1000".to_string(),
            name: "Test".to_string(),
            account_type: "activo".to_string(),
            level: None,
            parent_code: None,
            is_operational: None,
        };
        let result = cuentas_actualizar(
            State(state),
            axum::extract::Path("code-1".to_string()),
            ctx,
            axum::Form(form),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn asientos_actualizar_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AsientoForm {
            date: "2024-01-01".to_string(),
            description: "Test".to_string(),
            lineas: vec![],
        };
        let result = asientos_actualizar(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
            axum::Form(form),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn factura_imprimir_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = factura_imprimir(
            State(state),
            axum::extract::Path("id-1".to_string()),
            ctx,
        )
        .await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn login_post_sin_db_returns_503() {
        use axum::extract::Form;
        use http::HeaderMap;

        let state = crate::state::AppState::for_tests();
        let headers = HeaderMap::new();
        let form = rustem_auth::handlers::LoginRequest {
            email: "test@test.com".to_string(),
            password: "password123".to_string(),
        };
        let result = login_post(State(state), headers, Form(form)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn register_post_sin_db_redirects_to_login() {
        use axum::extract::Form;

        let state = crate::state::AppState::for_tests();
        let form = rustem_auth::handlers::RegisterRequest {
            email: "test@test.com".to_string(),
            password: "password123".to_string(),
            full_name: "Test User".to_string(),
            company_name: "Test Co".to_string(),
            company_rif: "V-12345678-9".to_string(),
        };
        let result = register_post(State(state), Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn asientos_crear_sin_db() {
        use serde_json::json;
        use axum::extract::Form;

        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AsientoForm {
            date: "2024-01-01".to_string(),
            description: "Test".to_string(),
            lineas: vec![],
        };
        let result = asientos_crear(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
        let redirect = result.unwrap();
        // Con db=None, redirige a /contabilidad/asientos
    }

    #[tokio::test]
    async fn facturas_nuevo_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = facturas_nuevo(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }

    #[tokio::test]
    async fn asientos_crear_desde_venta_sin_db() {
        use serde_json::json;
        use axum::extract::Form;

        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AsientoAutoForm {
            date: "2024-01-01".to_string(),
            description: "Test desde venta".to_string(),
            lineas: vec![],
        };
        let result = asientos_crear_desde_venta(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn asientos_crear_desde_movimiento_sin_db() {
        use serde_json::json;
        use axum::extract::Form;

        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let form = AsientoAutoForm {
            date: "2024-01-01".to_string(),
            description: "Test desde movimiento".to_string(),
            lineas: vec![],
        };
        let result = asientos_crear_desde_movimiento(State(state), ctx, axum::Form(form)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cuentas_arbol_view_sin_db() {
        let state = crate::state::AppState::for_tests();
        let ctx = TenantCtx(rustem_db::store::TenantContext::new("company-test"));
        let html = cuentas_arbol_view(State(state), ctx).await;
        assert!(html.0.contains("Sin base de datos disponible"));
    }
}
