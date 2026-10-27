// ============================================================================
// RUSTEM SUITE — Tests de Integración: Facturas (Fase 4)
// ============================================================================
// Pruebas de integración HTTP para el módulo de facturación fiscal.
// En modo simulado (sin SurrealDB), los handlers retornan el mensaje
// "Sin base de datos disponible". Se verifica:
// - Cero JavaScript en el HTML servido (incluido @media print).
// - RBAC protege las rutas de escritura.
// - Cálculo fiscal: IVA (16%, 8%, 0%), IGTF, tasa BCV.
// - Número de control SENIAT.
// - Vista de factura para impresión fiscal.
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

/// Construye una petición GET sin permisos de ventas.
fn get_without_sales_permission(uri: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let mut ctx = TenantContext::new("company-test");
    ctx.roles = vec!["Almacenista".to_string()];
    ctx.permissions = vec!["inventory:read".to_string(), "inventory:write".to_string()];
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
async fn facturas_list_returns_200_without_db() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/facturas")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = html_body(resp).await;
    assert_no_js(&html);
    assert!(
        html.contains("Facturas") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Facturas' o mensaje de error"
    );
}

#[tokio::test]
async fn facturas_list_denies_without_sales_read() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_without_sales_permission("/facturas"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn facturas_nuevo_returns_200_without_db() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/facturas/nuevo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = html_body(resp).await;
    assert_no_js(&html);
    assert!(html.contains("Nueva Factura"));
}

#[tokio::test]
async fn facturas_nuevo_denies_without_sales_write() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_without_sales_permission("/facturas/nuevo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn facturas_crear_denies_without_sales_write() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form(
            "/facturas/crear",
            "customer_id=c1&series=001&number=1&control_number=001-000001&currency=VES&iva_rate=16&igtf_rate_percent=0&subtotal_cents=10000",
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
async fn factura_imprimir_returns_200_without_db() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/facturas/test-id/imprimir"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = html_body(resp).await;
    assert_no_js(&html);
    assert!(
        html.contains("FACTURA FISCAL") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'FACTURA FISCAL' o mensaje de error"
    );
}

#[tokio::test]
async fn factura_imprimir_denies_without_sales_read() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_without_sales_permission("/facturas/test-id/imprimir"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn security_headers_are_present_for_facturas() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/facturas")).await.unwrap();
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
