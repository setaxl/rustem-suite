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
    use super::middleware::security_headers;

    #[test]
    fn web_security_headers_is_accessible() {
        // Verifica que el middleware de security headers es accesible desde
        // la raíz del crate y es un elemento de función válido.
        let _ = security_headers;
    }
}
