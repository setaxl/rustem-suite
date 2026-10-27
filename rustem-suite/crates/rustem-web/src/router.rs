// ============================================================================
// RUSTEM SUITE — rustem-web::router
// Router Axum: rutas, handlers, middlewares (tenant, rate limit, seguridad).
// ============================================================================

//! # Módulo `router`
//! Definición de rutas de la aplicación Axum:
//! - Páginas públicas: login, registro.
//! - Páginas protegidas: dashboard, módulos (con middleware de tenant).
//! - Rate limiting en POST /login (anti-fuerza bruta).
//! - Assets estáticos: CSS.

use axum::Router;
use axum::middleware::from_fn;
use axum::routing::{get, post};
use std::sync::Arc;
use tower_http::services::ServeDir;

use crate::middleware::{
    rate_limit_middleware, rbac_middleware, security_headers, tenant::tenant_middleware,
};
use crate::state::AppState;

/// Construye el router de la aplicación con todas las rutas y middlewares.
pub struct AppRouter {
    state: AppState,
}

impl AppRouter {
    /// Crea un nuevo router con el estado dado.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Devuelve el `Router` de Axum listo para servir.
    pub fn into_router(self) -> Router {
        let state = Arc::new(self.state);

        // Rutas públicas (sin tenant requerido).
        let public = Router::new()
            .route("/", get(crate::handlers::root))
            .route("/login", get(crate::handlers::login_get))
            .route(
                "/login",
                post(crate::handlers::login_post).layer(from_fn({
                    let state = Arc::clone(&state);
                    move |req, next| {
                        let state = (*state).clone();
                        async move { rate_limit_middleware(state, req, next).await }
                    }
                })),
            )
            .route("/registro", get(crate::handlers::register_get))
            .route("/registro", post(crate::handlers::register_post))
            .route("/logout", post(crate::handlers::logout_post));

        // Rutas protegidas (requieren tenant + permiso RBAC desde sesión).
        let dashboard = Router::new()
            .route("/dashboard", get(crate::handlers::dashboard))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("dashboard", "read", req, next).await
            }));

        // --- Almacenes (RBAC: inventory:read / inventory:write) ---
        let almacenes_list = Router::new()
            .route("/almacenes", get(crate::handlers::almacenes_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "read", req, next).await
            }));
        let almacenes_nuevo = Router::new()
            .route("/almacenes/nuevo", get(crate::handlers::almacenes_nuevo))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let almacenes_crear = Router::new()
            .route("/almacenes/crear", post(crate::handlers::almacenes_crear))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let almacenes_editar = Router::new()
            .route(
                "/almacenes/{id}/editar",
                get(crate::handlers::almacenes_editar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let almacenes_actualizar = Router::new()
            .route(
                "/almacenes/{id}/actualizar",
                post(crate::handlers::almacenes_actualizar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));

        // --- Productos (RBAC: inventory:read / inventory:write) ---
        let productos_list = Router::new()
            .route("/productos", get(crate::handlers::productos_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "read", req, next).await
            }));
        let productos_nuevo = Router::new()
            .route("/productos/nuevo", get(crate::handlers::productos_nuevo))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let productos_crear = Router::new()
            .route("/productos/crear", post(crate::handlers::productos_crear))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let productos_editar = Router::new()
            .route(
                "/productos/{id}/editar",
                get(crate::handlers::productos_editar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let productos_actualizar = Router::new()
            .route(
                "/productos/{id}/actualizar",
                post(crate::handlers::productos_actualizar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));

        // --- Movimientos de stock (RBAC: inventory:write) ---
        let movimientos_entrada = Router::new()
            .route(
                "/movimientos/entrada",
                post(crate::handlers::movimientos_entrada),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let movimientos_salida = Router::new()
            .route(
                "/movimientos/salida",
                post(crate::handlers::movimientos_salida),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let movimientos_ajuste = Router::new()
            .route(
                "/movimientos/ajuste",
                post(crate::handlers::movimientos_ajuste),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));
        let movimientos_transferencia = Router::new()
            .route(
                "/movimientos/transferencia",
                post(crate::handlers::movimientos_transferencia),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "write", req, next).await
            }));

        // --- Kardex (RBAC: inventory:read) ---
        let kardex = Router::new()
            .route("/kardex", get(crate::handlers::kardex))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "read", req, next).await
            }));

        // --- Clientes (RBAC: sales:read / sales:write) ---
        let clientes_list = Router::new()
            .route("/clientes", get(crate::handlers::clientes_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "read", req, next).await
            }));
        let clientes_nuevo = Router::new()
            .route("/clientes/nuevo", get(crate::handlers::clientes_nuevo))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));
        let clientes_crear = Router::new()
            .route("/clientes/crear", post(crate::handlers::clientes_crear))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));
        let clientes_editar = Router::new()
            .route(
                "/clientes/{id}/editar",
                get(crate::handlers::clientes_editar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));
        let clientes_actualizar = Router::new()
            .route(
                "/clientes/{id}/actualizar",
                post(crate::handlers::clientes_actualizar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));

        // --- Plan de Cuentas (RBAC: accounting:read / accounting:write) ---
        let cuentas_list = Router::new()
            .route("/contabilidad/cuentas", get(crate::handlers::cuentas_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "read", req, next).await
            }));
        let cuentas_nuevo = Router::new()
            .route(
                "/contabilidad/cuentas/nuevo",
                get(crate::handlers::cuentas_nuevo),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let cuentas_crear = Router::new()
            .route(
                "/contabilidad/cuentas/crear",
                post(crate::handlers::cuentas_crear),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let cuentas_editar = Router::new()
            .route(
                "/contabilidad/cuentas/{code}/editar",
                get(crate::handlers::cuentas_editar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let cuentas_actualizar = Router::new()
            .route(
                "/contabilidad/cuentas/{code}/actualizar",
                post(crate::handlers::cuentas_actualizar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let cuentas_arbol = Router::new()
            .route(
                "/contabilidad/cuentas/arbol",
                get(crate::handlers::cuentas_arbol_view),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "read", req, next).await
            }));

        // --- Asientos Contables (RBAC: accounting:read / accounting:write) ---
        let asientos_list = Router::new()
            .route(
                "/contabilidad/asientos",
                get(crate::handlers::asientos_list),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "read", req, next).await
            }));
        let asientos_nuevo = Router::new()
            .route(
                "/contabilidad/asientos/nuevo",
                get(crate::handlers::asientos_nuevo),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let asientos_crear = Router::new()
            .route(
                "/contabilidad/asientos/crear",
                post(crate::handlers::asientos_crear),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let asientos_editar = Router::new()
            .route(
                "/contabilidad/asientos/{id}/editar",
                get(crate::handlers::asientos_editar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));
        let asientos_actualizar = Router::new()
            .route(
                "/contabilidad/asientos/{id}/actualizar",
                post(crate::handlers::asientos_actualizar),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "write", req, next).await
            }));

        // --- Cotizaciones (RBAC: sales:read / sales:write) ---
        let cotizaciones_list = Router::new()
            .route("/cotizaciones", get(crate::handlers::cotizaciones_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "read", req, next).await
            }));
        let cotizaciones_nuevo = Router::new()
            .route(
                "/cotizaciones/nuevo",
                get(crate::handlers::cotizaciones_nuevo),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));
        let cotizaciones_crear = Router::new()
            .route(
                "/cotizaciones/crear",
                post(crate::handlers::cotizaciones_crear),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));

        // --- Pedidos (RBAC: sales:read) ---
        let pedidos_list = Router::new()
            .route("/pedidos", get(crate::handlers::pedidos_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "read", req, next).await
            }));

        // --- Facturas (RBAC: sales:read / sales:write) ---
        let facturas_list = Router::new()
            .route("/facturas", get(crate::handlers::facturas_list))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "read", req, next).await
            }));
        let facturas_nuevo = Router::new()
            .route("/facturas/nuevo", get(crate::handlers::facturas_nuevo))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));
        let facturas_crear = Router::new()
            .route("/facturas/crear", post(crate::handlers::facturas_crear))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "write", req, next).await
            }));
        let factura_imprimir = Router::new()
            .route(
                "/facturas/{id}/imprimir",
                get(crate::handlers::factura_imprimir),
            )
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "read", req, next).await
            }));

        let inventario = Router::new()
            .route("/inventario", get(crate::handlers::inventario))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("inventory", "read", req, next).await
            }));

        let ventas = Router::new()
            .route("/ventas", get(crate::handlers::ventas))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("sales", "read", req, next).await
            }));

        let contabilidad = Router::new()
            .route("/contabilidad", get(crate::handlers::contabilidad))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("accounting", "read", req, next).await
            }));

        let admin = Router::new()
            .route("/admin", get(crate::handlers::admin))
            .layer(axum::middleware::from_fn(move |req, next| async move {
                rbac_middleware("admin", "read", req, next).await
            }));

        let protected = dashboard
            .merge(inventario)
            .merge(almacenes_list)
            .merge(almacenes_nuevo)
            .merge(almacenes_crear)
            .merge(almacenes_editar)
            .merge(almacenes_actualizar)
            .merge(productos_list)
            .merge(productos_nuevo)
            .merge(productos_crear)
            .merge(productos_editar)
            .merge(productos_actualizar)
            .merge(movimientos_entrada)
            .merge(movimientos_salida)
            .merge(movimientos_ajuste)
            .merge(movimientos_transferencia)
            .merge(kardex)
            .merge(clientes_list)
            .merge(clientes_nuevo)
            .merge(clientes_crear)
            .merge(clientes_editar)
            .merge(clientes_actualizar)
            // --- Plan de Cuentas (RBAC: accounting:read / accounting:write) ---
            .merge(cuentas_list)
            .merge(cuentas_nuevo)
            .merge(cuentas_crear)
            .merge(cuentas_editar)
            .merge(cuentas_actualizar)
            .merge(cuentas_arbol)
            // --- Asientos Contables (RBAC: accounting:read / accounting:write) ---
            .merge(asientos_list)
            .merge(asientos_nuevo)
            .merge(asientos_crear)
            .merge(asientos_editar)
            .merge(asientos_actualizar)
            .merge(cotizaciones_list)
            .merge(cotizaciones_nuevo)
            .merge(cotizaciones_crear)
            .merge(pedidos_list)
            .merge(facturas_list)
            .merge(facturas_nuevo)
            .merge(facturas_crear)
            .merge(factura_imprimir)
            .merge(ventas)
            .merge(contabilidad)
            .merge(admin)
            .layer(from_fn({
                let state = Arc::clone(&state);
                move |req, next| {
                    let state = (*state).clone();
                    async move { tenant_middleware(state, req, next).await }
                }
            }));

        public
            .merge(protected)
            // Assets estáticos (CSS).
            .nest_service("/static", ServeDir::new("static"))
            // Cabeceras de seguridad en todas las respuestas.
            .layer(axum::middleware::from_fn(security_headers))
            .with_state((*state).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use axum::body::Body;
    use tower::ServiceExt;

    #[tokio::test]
    async fn router_serves_login_page() {
        let state = AppState::for_tests();
        let router = AppRouter::new(state).into_router();

        let req = http::Request::builder()
            .uri("/login")
            .body(Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn router_serves_dashboard() {
        let state = AppState::for_tests();
        let router = AppRouter::new(state).into_router();

        let req = http::Request::builder()
            .uri("/dashboard")
            .body(Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn router_404_for_unknown_route() {
        let state = AppState::for_tests();
        let router = AppRouter::new(state).into_router();

        let req = http::Request::builder()
            .uri("/ruta-inexistente")
            .body(Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 404);
    }
}
