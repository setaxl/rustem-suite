// ============================================================================
// RUSTEM SUITE — rustem-web::middleware::rate_limit
// Rate limiter para el endpoint de login (anti-fuerza bruta).
// ============================================================================

//! # Módulo `rate_limit`
//! Middleware que consulta el [`BruteForceGuard`] de `rustem-auth` para
//! bloquear IPs y usuarios tras 5 intentos fallidos consecutivos.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::StatusCode;
use tracing::warn;

use crate::state::AppState;
use rustem_auth::brute_force::BruteForceGuard;

/// Función de middleware standalone (usada con `from_fn_with_state`).
/// Recibe `AppState` y extrae el `BruteForceGuard` de él.
pub async fn rate_limit_middleware(
    state: AppState,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extraer IP del cliente (respetando proxy).
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .or_else(|| req.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Verificar bloqueo por IP.
    {
        let g = state.brute_force.read().await;
        if g.is_ip_locked(&ip) {
            let remaining = g.ip_remaining_seconds(&ip);
            warn!("Rate limit: IP {ip} bloqueada, quedan {remaining} segundos");
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    Ok(next.run(req).await)
}

/// Verifica y registra un intento fallido de login para IP y username.
/// Devuelve `Some(StatusCode)` si se debe rechazar (bloqueado), `None` si
/// se puede continuar.
pub fn check_and_record_failure(
    guard: &BruteForceGuard,
    ip: &str,
    username: &str,
) -> Option<StatusCode> {
    let blocked = guard.record_failure(ip, username);
    if blocked {
        let remaining = guard.ip_remaining_seconds(ip);
        warn!(
            "Rate limit: IP {ip} y usuario {username} bloqueados tras 5 intentos fallidos. \
             Quedan {remaining} segundos."
        );
        Some(StatusCode::TOO_MANY_REQUESTS)
    } else {
        None
    }
}

/// Registra un intento exitoso (reinicia contadores).
pub fn record_success(guard: &BruteForceGuard, ip: &str, username: &str) {
    guard.record_success(ip, username);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_and_record_failure_blocks_after_five() {
        let guard = BruteForceGuard::new();
        let ip = "192.168.1.50";
        let user = "testuser";

        for _ in 1..=4 {
            assert!(check_and_record_failure(&guard, ip, user).is_none());
        }
        assert!(check_and_record_failure(&guard, ip, user).is_some());
    }

    #[test]
    fn record_success_clears_block() {
        let guard = BruteForceGuard::new();
        let ip = "10.0.0.99";
        let user = "admin";

        for _ in 1..=5 {
            guard.record_failure(ip, user);
        }
        assert!(guard.is_ip_locked(ip));

        guard.record_success(ip, user);
        // Tras éxito, contador reiniciado.
        assert!(!guard.is_ip_locked(ip));
    }
}
