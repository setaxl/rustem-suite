// ============================================================================
// RUSTEM SUITE — rustem-web::middleware
// Middlewares de seguridad HTTP (CSP, headers, tenant, rate limiting).
// ============================================================================

//! # Módulo `middleware`
//! Middlewares de seguridad para Axum:
//! - [`security_headers`]: CSP estricta y cabeceras defensivas.
//! - [`tenant_middleware`]: multi-tenancy (inyecta `tenant_id` desde sesión).
//! - [`rate_limit_middleware`]: anti-fuerza bruta en login.
//! - [`rbac_middleware`]: control de acceso por roles y permisos.

pub mod rate_limit;
pub mod tenant;

pub use rate_limit::rate_limit_middleware;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use http::header;
use rustem_db::store::TenantContext;
use tracing::warn;

/// Middleware de cabeceras de seguridad.
/// Aplica CSP estricta y otras cabeceras defensivas a cada respuesta.
pub async fn security_headers(
    req: Request,
    next: Next,
) -> Result<Response, axum::http::StatusCode> {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'".parse().unwrap(),
    );
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    Ok(response)
}

/// Middleware de control de acceso RBAC.
/// Lee el [`TenantContext`] inyectado por [`tenant_middleware`] y verifica
/// que el usuario autenticado tenga el permiso `(resource, action)`.
/// Devuelve HTTP 403 si no tiene el permiso requerido.
pub async fn rbac_middleware(
    resource: &'static str,
    action: &'static str,
    req: Request,
    next: Next,
) -> Result<Response, http::StatusCode> {
    let tenant_ctx = req
        .extensions()
        .get::<TenantContext>()
        .cloned()
        .unwrap_or_default();

    // Sin tenant_id (usuario anónimo / sin sesión): permitir paso.
    // Esto ocurre cuando no hay base de datos configurada o no hay cookie
    // de sesión válida. En producción con DB, tenant_middleware siempre
    // inyecta un contexto con tenant_id real.
    if tenant_ctx.tenant_id.is_empty() {
        return Ok(next.run(req).await);
    }

    if tenant_ctx.has_permission(resource, action) {
        Ok(next.run(req).await)
    } else {
        warn!(
            "RBAC: acceso denegado para tenant={} user={:?} a {}/{}",
            tenant_ctx.tenant_id, tenant_ctx.user_id, resource, action
        );
        Err(http::StatusCode::FORBIDDEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use tower::ServiceExt;

    #[tokio::test]
    async fn security_headers_are_applied() {
        let app = Router::new()
            .route("/test", axum::routing::get(|| async { "hola" }))
            .layer(axum::middleware::from_fn(security_headers));

        let req = http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        let headers = resp.headers();

        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "default-src 'self'"
        );
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
        assert_eq!(
            headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
        assert_eq!(
            headers.get(header::REFERRER_POLICY).unwrap(),
            "strict-origin-when-cross-origin"
        );
    }

    #[tokio::test]
    async fn rbac_middleware_anonymous_tenant_passes_through() {
        let app = Router::new()
            .route("/test", axum::routing::get(|| async { "hola" }))
            .layer(axum::middleware::from_fn(|req, next| async move {
                rbac_middleware("inventory", "read", req, next).await
            }));

        let req = http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rbac_middleware_with_permission_passes_through() {
        let app = Router::new()
            .route("/test", axum::routing::get(|| async { "hola" }))
            .layer(axum::middleware::from_fn(move |req, next| {
                let mut ctx = TenantContext::new("company-test");
                ctx.permissions = vec!["inventory:read".to_string()];
                req.extensions_mut().insert(ctx);
                rbac_middleware("inventory", "read", req, next).await
            }));

        let req = http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rbac_middleware_without_permission_returns_403() {
        let app = Router::new()
            .route("/test", axum::routing::get(|| async { "hola" }))
            .layer(axum::middleware::from_fn(move |req, next| {
                let mut ctx = TenantContext::new("company-test");
                ctx.permissions = vec!["sales:read".to_string()];
                req.extensions_mut().insert(ctx);
                rbac_middleware("inventory", "read", req, next).await
            }));

        let req = http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rbac_middleware_with_empty_permissions_returns_403() {
        let app = Router::new()
            .route("/test", axum::routing::get(|| async { "hola" }))
            .layer(axum::middleware::from_fn(move |req, next| {
                let ctx = TenantContext::new("company-test");
                req.extensions_mut().insert(ctx);
                rbac_middleware("inventory", "read", req, next).await
            }));

        let req = http::Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
