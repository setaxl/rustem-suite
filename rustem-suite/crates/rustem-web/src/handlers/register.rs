// Handler GET /registro — muestra el formulario de registro.
pub async fn register_get() -> axum::response::Html<String> {
    axum::response::Html("<h1>Registro</h1>".to_string())
}

// Handler POST /registro — procesa el registro.
pub async fn register_post() -> axum::response::Redirect {
    axum::response::Redirect::to("/login")
}