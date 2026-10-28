// ============================================================================
// RUSTEM SUITE — rustem-web::middleware::tenant
// Middleware de multi-tenancy: extrae el tenant_id de la sesión activa y lo
// inyecta en las extensiones de la request de Axum.
// ============================================================================

//! # Módulo `tenant`
//! Middleware de multi-tenancy: extrae el `tenant_id` de la sesión activa y lo
//! inyecta en las extensiones de la request de Axum.
//!
//! El middleware:
//! 1. Lee la cookie de sesión (`session_token`).
//! 2. Valida el token contra SurrealDB (hash SHA-256).
//! 3. Extrae el `tenant_id` (company_id) de la sesión.
//! 4. Inserta un [`TenantContext`] en `req.extensions()` para que los handlers
//!    y `rustem-db::SurrealStore` lo usen automáticamente.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::header;
use serde_json::Value;
use tracing::{debug, warn};

use crate::state::AppState;
use rustem_auth::session::SessionToken;
use rustem_db::DbConnection;
use rustem_db::store::TenantContext;

/// Nombre de la cookie de sesión.
pub const SESSION_COOKIE_NAME: &str = "session_token";

/// Convierte un vector de pares (nombre, valor) a un objeto JSON para parámetros de SurrealDB.
fn params_to_value(params: Vec<(String, serde_json::Value)>) -> serde_json::Value {
    let mut map = serde_json::Map::with_capacity(params.len());
    for (key, value) in params {
        map.insert(key, value);
    }
    serde_json::Value::Object(map)
}

/// Datos de sesión resueltos (extraídos de SurrealDB).
#[derive(Debug, Clone)]
pub struct ResolvedSession {
    pub user_id: String,
    pub company_id: Option<String>,
}

/// Resuelve la sesión a partir del token (hash SHA-256).
async fn resolve_session(
    conn: &dyn DbConnection,
    token: &str,
) -> anyhow::Result<Option<ResolvedSession>> {
    let token_hash = SessionToken(token.to_string()).hash();

    let sql = r#"
        SELECT * FROM session
        WHERE token_hash = $token_hash
          AND expires_at > $now
        LIMIT 1"#;

    let params: Vec<(String, serde_json::Value)> = vec![
        (
            "$token_hash".to_string(),
            serde_json::Value::String(token_hash),
        ),
        (
            "$now".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        ),
    ];

    let mut resp = conn.query(sql, params_to_value(params)).await?;
    let sessions: Vec<serde_json::Value> = resp.take(0)?;

    if let Some(s) = sessions.into_iter().next() {
        let user_id = s
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let company_id = s
            .get("company_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Some(ResolvedSession {
            user_id,
            company_id,
        }))
    } else {
        Ok(None)
    }
}

/// Función de middleware standalone.
/// Recibe `AppState` (clonado desde un `Arc`), valida la sesión e inyecta
/// el `TenantContext` en las extensiones de la request.
pub async fn tenant_middleware(
    state: AppState,
    req: Request,
    next: Next,
) -> Result<Response, http::StatusCode> {
    let conn = match &state.db {
        Some(c) => c.clone(),
        None => {
            // Sin base de datos: permitir paso para rutas públicas.
            return Ok(next.run(req).await);
        }
    };

    let conn_read = conn.read().await;

    // Extraer cookie de sesión.
    let session_token = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_session_cookie);

    let tenant_ctx = match session_token {
        Some(token) => match resolve_session(conn_read.as_ref(), &token).await {
            Ok(Some(session)) => {
                debug!(
                    "Sesión resuelta: user={} company={:?}",
                    session.user_id, session.company_id
                );

                // Resolver roles y permisos del usuario mediante grafo de SurrealDB.
                let (roles, permissions) =
                    resolve_user_roles_and_permissions(conn_read.as_ref(), &session.user_id)
                        .await
                        .unwrap_or_default();

                TenantContext {
                    tenant_id: session.company_id.clone().unwrap_or_default(),
                    user_id: Some(session.user_id),
                    roles,
                    permissions,
                    ip_address: req
                        .headers()
                        .get("x-forwarded-for")
                        .or_else(|| req.headers().get("x-real-ip"))
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string()),
                    user_agent: req
                        .headers()
                        .get(header::USER_AGENT)
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string()),
                }
            }
            Ok(None) => {
                warn!("Sesión inválida o expirada");
                TenantContext::new("".to_string())
            }
            Err(e) => {
                warn!("Error resolviendo sesión: {e}");
                TenantContext::new("".to_string())
            }
        },
        None => {
            // No hay cookie de sesión: contexto vacío (usuario anónimo).
            TenantContext::new("".to_string())
        }
    };

    // Insertar el contexto en las extensiones de la request.
    let mut req = req;
    req.extensions_mut().insert(tenant_ctx);

    Ok(next.run(req).await)
}

/// Parsea el valor de la cookie `Cookie: session_token=xxx; ...` y devuelve
/// el valor del token.
fn parse_session_cookie(cookie_header: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=')
            && key == SESSION_COOKIE_NAME
        {
            return Some(value.to_string());
        }
    }
    None
}

/// Resuelve los roles y permisos de un usuario a través del grafo de SurrealDB:
/// `user -> HAS_ROLE -> role -> CAN_ACCESS -> permission`.
/// Devuelve vectores vacíos si no hay datos o falla la consulta.
async fn resolve_user_roles_and_permissions(
    conn: &dyn DbConnection,
    user_id: &str,
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let conn_read = conn;

    // Roles del usuario: user -> HAS_ROLE -> role
    let sql_roles = r#"
        SELECT role.name AS role_name
        FROM (SELECT * FROM user WHERE id = $user_id LIMIT 1)
        ->HAS_ROLE->
        role
    "#;
    let params: Vec<(String, Value)> =
        vec![("$user_id".to_string(), Value::String(user_id.to_string()))];
    let roles = match conn_read.query(sql_roles, params_to_value(params.clone())).await {
        Ok(mut resp) => resp
            .take::<Vec<Value>>(0)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| {
                v.get("role_name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    // Permisos del usuario: user -> HAS_ROLE -> role -> CAN_ACCESS -> permission
    let sql_perms = r#"
        SELECT permission.resource AS resource, permission.action AS action
        FROM (SELECT * FROM user WHERE id = $user_id LIMIT 1)
        ->HAS_ROLE->
        role
        ->CAN_ACCESS->
        permission
    "#;
    let permissions = match conn_read.query(sql_perms, params_to_value(params)).await {
        Ok(mut resp) => resp
            .take::<Vec<Value>>(0)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| {
                let resource = v.get("resource").and_then(|r| r.as_str())?;
                let action = v.get("action").and_then(|a| a.as_str())?;
                Some(format!("{resource}:{action}"))
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    Ok((roles, permissions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_cookie_extracts_token() {
        let header = "session_token=abc123; other=xyz";
        assert_eq!(parse_session_cookie(header), Some("abc123".to_string()));
    }

    #[test]
    fn parse_session_cookie_missing() {
        let header = "other=xyz";
        assert_eq!(parse_session_cookie(header), None);
    }

    #[test]
    fn parse_session_cookie_empty() {
        assert_eq!(parse_session_cookie(""), None);
    }

    use std::sync::Arc;
    use anyhow::anyhow;
    use rustem_db::MigrationResult;

    /// Mock de DbConnection para pruebas sin SurrealDB real.
    /// Siempre devuelve error en query, probando los paths de manejo de error.
    struct MockDbConnection;

    #[async_trait::async_trait]
    impl DbConnection for MockDbConnection {
        async fn query(&self, _sql: &str, _params: serde_json::Value) -> anyhow::Result<rustem_db::DbResponse> {
            Err(anyhow!("mock query error"))
        }

        async fn execute(&self, _sql: &str, _params: serde_json::Value) -> anyhow::Result<rustem_db::DbResponse> {
            Err(anyhow!("mock execute error"))
        }

        async fn apply_migrations(&self) -> anyhow::Result<MigrationResult> {
            Ok(MigrationResult { applied: 0, skipped: 0 })
        }

        fn clone_box(&self) -> Arc<dyn DbConnection> {
            Arc::new(Self)
        }
    }

    #[tokio::test]
    async fn resolve_session_error() {
        let mock = Arc::new(MockDbConnection);
        let result = resolve_session(mock.as_ref(), "valid-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resolve_user_roles_and_permissions_error() {
        let mock = Arc::new(MockDbConnection);
        let (roles, permissions) = resolve_user_roles_and_permissions(mock.as_ref(), "user-1")
            .await
            .unwrap_or_default();
        assert!(roles.is_empty());
        assert!(permissions.is_empty());
    }
}
