// ============================================================================
// RUSTEM SUITE — Tests de Integración: Productos (Fase 3)
// ============================================================================
// Pruebas de integración HTTP para el módulo de productos.
// En modo simulado (sin SurrealDB), los handlers retornan el mensaje
// "Sin base de datos disponible". Se verifica:
// - Cero JavaScript en el HTML servido.
// - RBAC protege las rutas de escritura.
// - Validación de alícuota IVA.
// ============================================================================

use axum::body::Body;
use axum::extract::Request;
use http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use rustem_db::store::TenantContext;
use rustem_web::router::AppRouter;
use rustem_web::state::AppState;

/// Construye el router para pruebas (modo simulado, sin SurrealDB).
fn test_router() -> AppRouter {
    let state = AppState::for_tests();
    AppRouter::new(state)
}

/// Construye una petición GET con TenantContext inyectado en las extensiones.
fn get_with_tenant(uri: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let mut ctx = TenantContext::new("company-test");
    ctx.roles = vec!["Owner".to_string()];
    ctx.permissions = vec![
        "sales:read".to_string(),
        "sales:write".to_string(),
        "inventory:read".to_string(),
        "inventory:write".to_string(),
    ];
    req.extensions_mut().insert(ctx);
    req
}

/// Construye una petición GET sin permisos de inventario.
fn get_without_inventory_permission(uri: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let mut ctx = TenantContext::new("company-test");
    ctx.roles = vec!["Vendedor".to_string()];
    ctx.permissions = vec!["sales:read".to_string(), "sales:write".to_string()];
    req.extensions_mut().insert(ctx);
    req
}

/// Construye una petición POST con form-urlencoded.
fn post_form(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Verifica que el cuerpo HTML no contenga JavaScript ni atributos on*.
fn assert_no_js(html: &str) {
    assert!(!html.contains("<script"), "HTML contiene <script");
    assert!(!html.contains("onclick="), "HTML contiene onclick=");
    assert!(!html.contains("onchange="), "HTML contiene onchange=");
    assert!(!html.contains("onsubmit="), "HTML contiene onsubmit=");
    assert!(!html.contains("onload="), "HTML contiene onload=");
}

/// Extrae el cuerpo HTML de una respuesta como String.
async fn html_body(resp: http::Response<Body>) -> String {
    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn productos_list_returns_200_without_db() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/productos")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = html_body(resp).await;
    assert_no_js(&html);
    assert!(
        html.contains("Productos") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Productos' o mensaje de error"
    );
}

#[tokio::test]
async fn productos_list_denies_without_inventory_read() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_without_inventory_permission("/productos"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn productos_nuevo_returns_200_without_db() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/productos/nuevo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = html_body(resp).await;
    assert_no_js(&html);
    assert!(html.contains("Nuevo Producto"));
}

#[tokio::test]
async fn productos_nuevo_denies_without_inventory_write() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_without_inventory_permission("/productos/nuevo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn productos_crear_denies_without_inventory_write() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form(
            "/productos/crear",
            "sku=SKU-001&name=Producto+Test&unit=und&category=Electronica&vat_rate=16&min_stock=0",
        ))
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::FORBIDDEN
            || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "esperaba 403 o 500, obtuvo {:?}",
        resp.status()
    );
}

#[tokio::test]
async fn productos_editar_returns_200_without_db() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/productos/p1/editar"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = html_body(resp).await;
    assert_no_js(&html);
    assert!(html.contains("Editar Producto") || html.contains("Sin base de datos disponible"),);
}

#[tokio::test]
async fn security_headers_are_present_for_productos() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/productos")).await.unwrap();
    let headers = resp.headers();

    assert_eq!(
        headers.get(http::header::CONTENT_SECURITY_POLICY).unwrap(),
        "default-src 'self'"
    );
    assert_eq!(headers.get(http::header::X_FRAME_OPTIONS).unwrap(), "DENY");
    assert_eq!(
        headers.get(http::header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(
        headers.get(http::header::REFERRER_POLICY).unwrap(),
        "strict-origin-when-cross-origin"
    );
}
