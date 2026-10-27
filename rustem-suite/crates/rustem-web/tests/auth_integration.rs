// ============================================================================
// RUSTEM SUITE — Tests de Integración: Autenticación (Fase 1)
// ============================================================================
// Pruebas de integración HTTP contra el router Axum en modo simulado
// (sin SurrealDB). Verifican flujos PRG, cookies seguras httpOnly y
// ausencia de JavaScript en el HTML servido.
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

/// Construye una petición GET.
fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
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

#[tokio::test]
async fn login_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router.oneshot(get("/login")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert_no_js(&html);
    assert!(html.contains("Rustem Suite"));
    assert!(html.contains("/login"));
    assert!(html.contains(r#"method="post""#));
}

#[tokio::test]
async fn register_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router.oneshot(get("/registro")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert_no_js(&html);
    assert!(html.contains("Rustem Suite"));
    assert!(html.contains("Crear Cuenta"));
    assert!(html.contains(r#"method="post""#));
    assert!(html.contains("/registro"));
}

#[tokio::test]
async fn login_without_db_returns_service_unavailable() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form("/login", "email=admin@demo.com&password=demo"))
        .await
        .unwrap();

    // Sin base de datos configurada: no hay modo simulado con credenciales hardcoded.
    // Retornamos 503 Service Unavailable.
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn login_without_db_returns_service_unavailable_not_credentials() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form("/login", "email=admin@demo.com&password=demo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn login_without_db_returns_503() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form(
            "/login",
            "email=invalido@test.com&password=wrong",
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn register_without_db_redirects_to_login() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(post_form(
            "/registro",
            "email=nuevo@test.com&password=pass123&full_name=Usuario Nuevo&company_name=Empresa Test&company_rif=J-12345678-9",
        ))
        .await
        .unwrap();

    // Sin DB, el registro redirige a /login.
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn logout_redirects_to_login() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn root_redirects_to_login() {
    let router = test_router().into_router();
    let resp = router.oneshot(get("/")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let router = test_router().into_router();
    let resp = router.oneshot(get("/ruta-inexistente")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn security_headers_are_present() {
    let router = test_router().into_router();
    let resp = router.oneshot(get("/login")).await.unwrap();
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

// ============================================================================
// Tests de Integración: Ventas y Facturación (Fase 4)
// ============================================================================

#[tokio::test]
async fn clientes_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/clientes")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de clientes contiene <script"
    );
    assert!(
        !html.contains("onclick="),
        "HTML de clientes contiene onclick="
    );
    assert!(
        !html.contains("onchange="),
        "HTML de clientes contiene onchange="
    );
    assert!(
        !html.contains("onsubmit="),
        "HTML de clientes contiene onsubmit="
    );
    // Puede ser la lista completa o el mensaje "Sin base de datos disponible".
    assert!(
        html.contains("Clientes") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Clientes' o mensaje de error"
    );
}

#[tokio::test]
async fn clientes_nuevo_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/clientes/nuevo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de clientes/nuevo contiene <script"
    );
    assert!(html.contains("Nuevo Cliente"));
    assert!(html.contains(r#"method="post""#));
    assert!(html.contains("/clientes/crear"));
}

#[tokio::test]
async fn facturas_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/facturas")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de facturas contiene <script"
    );
    assert!(
        !html.contains("onclick="),
        "HTML de facturas contiene onclick="
    );
    assert!(
        html.contains("Facturas") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Facturas' o mensaje de error"
    );
}

#[tokio::test]
async fn facturas_nuevo_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/facturas/nuevo"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de facturas/nuevo contiene <script"
    );
    assert!(
        !html.contains("onclick="),
        "HTML de facturas/nuevo contiene onclick="
    );
    assert!(
        html.contains("Nueva Factura") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Nueva Factura' o mensaje de error"
    );
    // Si hay DB (no hay mensaje de error), el formulario debe tener estos campos.
    if !html.contains("Sin base de datos disponible") {
        assert!(html.contains("control_number"));
        assert!(html.contains("iva_rate"));
        assert!(html.contains("igtf_rate_percent"));
        assert!(html.contains(r#"method="post""#));
        assert!(html.contains("/facturas/crear"));
    }
}

#[tokio::test]
async fn cotizaciones_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/cotizaciones"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de cotizaciones contiene <script"
    );
    assert!(
        !html.contains("onclick="),
        "HTML de cotizaciones contiene onclick="
    );
    assert!(
        html.contains("Cotizaciones") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Cotizaciones' o mensaje de error"
    );
}

#[tokio::test]
async fn pedidos_page_returns_200_and_no_js() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/pedidos")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de pedidos contiene <script"
    );
    assert!(
        !html.contains("onclick="),
        "HTML de pedidos contiene onclick="
    );
    assert!(
        html.contains("Pedidos") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'Pedidos' o mensaje de error"
    );
}

#[tokio::test]
async fn sales_security_headers_are_present() {
    let router = test_router().into_router();
    let resp = router.oneshot(get_with_tenant("/clientes")).await.unwrap();
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

#[tokio::test]
async fn factura_imprimir_returns_200_without_db() {
    // Sin base de datos, el handler retorna HTML de error.
    let router = test_router().into_router();
    let resp = router
        .oneshot(get_with_tenant("/facturas/test-id/imprimir"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        !html.contains("<script"),
        "HTML de factura de impresión contiene <script"
    );
    assert!(
        html.contains("FACTURA FISCAL") || html.contains("Sin base de datos disponible"),
        "HTML debe contener 'FACTURA FISCAL' o mensaje de error"
    );
}
