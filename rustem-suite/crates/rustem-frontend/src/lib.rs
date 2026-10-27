// ============================================================================
// RUSTEM SUITE — Crate rustem-frontend
// Componentes y vistas Leptos (SSR puro, cero JS en cliente).
// ============================================================================

//! # Crate `rustem-frontend`
//! Vistas renderizadas con Leptos en modo SSR puro:
//! - Layout maestro HTML5 sin hidratación WASM.
//! - Vistas de login, dashboard, formularios CRUD.
//! - Plantilla de impresión fiscal (`@media print`).

pub mod components;
pub mod views;

pub use components::{Layout, flash, layout, render_ssr};
pub use views::{AccountNode, cuentas_arbol_view};
pub use views::{customer_form_view, dashboard_view, login_view, not_found_view, register_view};

#[cfg(test)]
mod tests {
    #[test]
    fn frontend_smoke_test() {
        let _ = ();
    }
}
