// ============================================================================
// RUSTEM SUITE — rustem-auth::handlers
// Tipos de solicitud y respuesta para flujo de autenticación (PRG).
// ============================================================================

//! # Módulo `handlers`
//! Tipos de datos para los handlers de autenticación (login, registro,
//! logout). Los handlers concretos de Axum viven en `rustem-web`.

use serde::{Deserialize, Serialize};

/// Solicitud de inicio de sesión.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Solicitud de registro de usuario.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
}

/// Respuesta de autenticación exitosa (contiene el token de sesión solo en
/// el cuerpo para pruebas; en producción se envía vía cookie httpOnly).
#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub email: String,
    pub session_token: Option<String>,
}

/// Mensaje flash para mostrar al usuario tras una redirección PRG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlashMessage {
    Success(String),
    Error(String),
    Info(String),
}

impl FlashMessage {
    pub fn success(msg: impl Into<String>) -> Self {
        FlashMessage::Success(msg.into())
    }
    pub fn error(msg: impl Into<String>) -> Self {
        FlashMessage::Error(msg.into())
    }
    pub fn info(msg: impl Into<String>) -> Self {
        FlashMessage::Info(msg.into())
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            FlashMessage::Success(_) => "flash-success",
            FlashMessage::Error(_) => "flash-error",
            FlashMessage::Info(_) => "flash-info",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_request_serialization() {
        let req = LoginRequest {
            email: "user@example.com".to_string(),
            password: "secret".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("user@example.com"));
        assert!(json.contains("secret"));
    }

    #[test]
    fn flash_message_css_class() {
        assert_eq!(FlashMessage::success("ok").css_class(), "flash-success");
        assert_eq!(FlashMessage::error("bad").css_class(), "flash-error");
        assert_eq!(FlashMessage::info("note").css_class(), "flash-info");
    }
}
