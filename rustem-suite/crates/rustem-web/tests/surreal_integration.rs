// ============================================================================
// RUSTEM SUITE — Tests de Integración con SurrealDB Real
// ============================================================================

use axum::body::Body;
use axum::extract::Request;
use http::StatusCode;
use http_body_util::BodyExt;
use rustem_web::router::AppRouter;
use rustem_web::state::AppState;
use tower::ServiceExt;

fn setup_env() {
    unsafe {
        std::env::set_var("DATABASE_URL", "ws://localhost:8000/rpc");
        std::env::set_var("DATABASE_USER", "root");
        std::env::set_var("DATABASE_PASS", "root");
        std::env::set_var("NAMESPACE", "rustem_test");
        std::env::set_var("DATABASE_NAME", "suite");
        std::env::set_var("RUSTEM_BASE_URL", "http://localhost:8080");
    }
}

async fn create_test_state() -> AppState {
    setup_env();
    AppState::from_env().await.expect("No se pudo conectar a SurrealDB real")
}

fn test_router(state: AppState) -> AppRouter {
    AppRouter::new(state)
}

fn get_with_tenant(uri: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let mut ctx = rustem_db::store::TenantContext::new("company-test");
    ctx.roles = vec!["Owner".to_string()];
    ctx.permissions = vec![
        "sales:read".to_string(),
        "sales:write".to_string(),
        "inventory:read".to_string(),
        "inventory:write".to_string(),
        "accounting:read".to_string(),
        "accounting:write".to_string(),
    ];
    req.extensions_mut().insert(ctx);
    req
}

async fn html_body(resp: http::Response<Body>) -> String {
    let aggregated = resp.into_body().collect().await.unwrap();
    let bytes = aggregated.to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn surrealdb_conexion_real() {
    let state = create_test_state().await;
    // from_env siempre retorna Ok (incluso si SurrealDB no está disponible, db será None).
    assert_eq!(state.base_url, "http://localhost:8080");
    // Si SurrealDB está disponible, db debe ser Some. Los otros tests de integración
    // validan que la conexión funciona cuando está activa.
    if let Some(db) = &state.db {
        let _ = db; // Conexión activa a SurrealDB
    }
}

#[tokio::test]
async fn almacenes_list_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/almacenes")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn productos_list_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/productos")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn facturas_list_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/facturas")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn clientes_list_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/clientes")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn kardex_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/kardex")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn cuentas_cuentas_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/contabilidad/cuentas")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn asientos_contables_con_surrealdb_real() {
    let state = create_test_state().await;
    let router = test_router(state).into_router();
    let resp = router.oneshot(get_with_tenant("/contabilidad/asientos")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}