// ============================================================================
// RUSTEM SUITE — rustem-web::middleware
// Middlewares de seguridad HTTP.
// ============================================================================

//! # Módulo `middleware`
//! Middlewares de seguridad para Axum.

pub mod security_headers;
pub mod tenant;
pub mod rate_limit;

// Placeholder
pub fn dummy() {}