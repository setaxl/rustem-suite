// ============================================================================
// RUSTEM SUITE — rustem-auth::brute_force
// Protección anti-fuerza bruta: bloqueo temporal de IP/usuario tras 5 intentos
// fallidos consecutivos.
// ============================================================================

//! # Módulo `brute_force`
//! Rate limiter y anti-fuerza bruta para el endpoint de login.
//! Rastrea intentos fallidos por dirección IP y por nombre de usuario.
//! Tras 5 intentos fallidos consecutivos, bloquea temporalmente la cuenta/IP.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Número máximo de intentos fallidos antes de bloquear.
const MAX_FAILED_ATTEMPTS: u32 = 5;

/// Duración del bloqueo tras exceder el límite: 15 minutos.
const LOCKOUT_DURATION_SECONDS: u64 = 15 * 60;

/// Registro de intentos fallidos para una clave (IP o usuario).
#[derive(Debug, Clone, Default)]
struct FailedAttempts {
    count: u32,
    first_failed_at: Option<Instant>,
    locked_until: Option<Instant>,
}

impl FailedAttempts {
    /// Registra un intento fallido. Devuelve `true` si la clave debe bloquearse.
    fn record_failure(&mut self) -> bool {
        let now = Instant::now();

        // Si ya estaba bloqueado y el bloqueo expiró, reiniciamos.
        if let Some(locked_until) = self.locked_until {
            if now >= locked_until {
                self.reset();
            } else {
                // Sigue bloqueado.
                return false;
            }
        }

        self.count += 1;
        if self.first_failed_at.is_none() {
            self.first_failed_at = Some(now);
        }

        if self.count >= MAX_FAILED_ATTEMPTS {
            self.locked_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECONDS));
            true
        } else {
            false
        }
    }

    /// Registra un intento exitoso (reinicia el contador).
    fn record_success(&mut self) {
        self.reset();
    }

    /// Verifica si la clave está actualmente bloqueada.
    fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            if Instant::now() >= locked_until {
                return false;
            }
            return true;
        }
        false
    }

    /// Devuelve los segundos restantes de bloqueo.
    fn remaining_lockout_seconds(&self) -> u64 {
        if let Some(locked_until) = self.locked_until {
            let now = Instant::now();
            if now < locked_until {
                return locked_until.duration_since(now).as_secs();
            }
        }
        0
    }

    fn reset(&mut self) {
        self.count = 0;
        self.first_failed_at = None;
        self.locked_until = None;
    }
}

/// Guard de anti-fuerza bruta. Thread-safe mediante `Mutex`.
pub struct BruteForceGuard {
    attempts_by_ip: Mutex<HashMap<String, FailedAttempts>>,
    attempts_by_user: Mutex<HashMap<String, FailedAttempts>>,
}

impl BruteForceGuard {
    /// Crea un nuevo guard vacío.
    pub fn new() -> Self {
        Self {
            attempts_by_ip: Mutex::new(HashMap::new()),
            attempts_by_user: Mutex::new(HashMap::new()),
        }
    }

    /// Registra un intento fallido para una IP y un usuario.
    /// Devuelve `true` si se debe bloquear (límite alcanzado).
    pub fn record_failure(&self, ip: &str, username: &str) -> bool {
        let ip_blocked = {
            let mut map = self.attempts_by_ip.lock().unwrap();
            let entry = map.entry(ip.to_string()).or_default();
            entry.record_failure()
        };

        let user_blocked = {
            let mut map = self.attempts_by_user.lock().unwrap();
            let entry = map.entry(username.to_string()).or_default();
            entry.record_failure()
        };

        ip_blocked || user_blocked
    }

    /// Registra un intento exitoso (reinicia contadores para IP y usuario).
    pub fn record_success(&self, ip: &str, username: &str) {
        if let Ok(mut map) = self.attempts_by_ip.lock()
            && let Some(entry) = map.get_mut(ip)
        {
            entry.record_success();
        }
        if let Ok(mut map) = self.attempts_by_user.lock()
            && let Some(entry) = map.get_mut(username)
        {
            entry.record_success();
        }
    }

    /// Verifica si una IP está bloqueada.
    pub fn is_ip_locked(&self, ip: &str) -> bool {
        self.attempts_by_ip
            .lock()
            .unwrap()
            .get(ip)
            .map(|e| e.is_locked())
            .unwrap_or(false)
    }

    /// Verifica si un usuario está bloqueado.
    pub fn is_user_locked(&self, username: &str) -> bool {
        self.attempts_by_user
            .lock()
            .unwrap()
            .get(username)
            .map(|e| e.is_locked())
            .unwrap_or(false)
    }

    /// Devuelve los segundos restantes de bloqueo para una IP.
    pub fn ip_remaining_seconds(&self, ip: &str) -> u64 {
        self.attempts_by_ip
            .lock()
            .unwrap()
            .get(ip)
            .map(|e| e.remaining_lockout_seconds())
            .unwrap_or(0)
    }

    /// Devuelve los segundos restantes de bloqueo para un usuario.
    pub fn user_remaining_seconds(&self, username: &str) -> u64 {
        self.attempts_by_user
            .lock()
            .unwrap()
            .get(username)
            .map(|e| e.remaining_lockout_seconds())
            .unwrap_or(0)
    }
}

impl Default for BruteForceGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_failure_blocks_after_max_attempts() {
        let guard = BruteForceGuard::new();
        let ip = "192.168.1.1";
        let user = "user1";

        // Los primeros 4 fallos no bloquean.
        for i in 1..=4 {
            assert!(
                !guard.record_failure(ip, user),
                "el fallo {i} no debe bloquear"
            );
        }
        assert!(!guard.is_ip_locked(ip));
        assert!(!guard.is_user_locked(user));

        // El 5vo fallo bloquea.
        assert!(guard.record_failure(ip, user));
        assert!(guard.is_ip_locked(ip));
        assert!(guard.is_user_locked(user));
        assert!(guard.ip_remaining_seconds(ip) > 0);
        assert!(guard.user_remaining_seconds(user) > 0);
    }

    #[test]
    fn record_success_resets_counter() {
        let guard = BruteForceGuard::new();
        let ip = "10.0.0.1";
        let user = "admin";

        // 3 fallos.
        for _ in 1..=3 {
            guard.record_failure(ip, user);
        }
        assert!(!guard.is_ip_locked(ip));

        // Un éxito reinicia.
        guard.record_success(ip, user);
        assert!(!guard.is_ip_locked(ip));

        // Después de un éxito, se necesitan 5 fallos de nuevo.
        for _i in 1..=4 {
            assert!(!guard.record_failure(ip, user));
        }
        assert!(!guard.is_ip_locked(ip));
        assert!(guard.record_failure(ip, user));
        assert!(guard.is_ip_locked(ip));
    }

    #[test]
    fn independent_keys() {
        let guard = BruteForceGuard::new();
        // Bloquear usuario "user1" no bloquea IP diferente.
        for _ in 1..=5 {
            guard.record_failure("1.1.1.1", "user1");
        }
        assert!(guard.is_user_locked("user1"));
        assert!(!guard.is_ip_locked("2.2.2.2"));
    }
}
