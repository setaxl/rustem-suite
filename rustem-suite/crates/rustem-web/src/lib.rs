// ============================================================================
// RUSTEM SUITE — Crate rustem-web (lib)
// Servidor Axum: rutas, handlers, middlewares y estáticos.
// ============================================================================

pub mod handlers;
pub mod middleware;
pub mod router;
pub mod state;

#[cfg(test)]
mod tests {
    #[test]
    fn web_smoke_test() {
        let _ = ();
    }
}
