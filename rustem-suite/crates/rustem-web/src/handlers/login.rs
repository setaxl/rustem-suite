// Handler GET / — redirige al login.
pub async fn root() -> axum::response::Redirect {
    axum::response::Redirect::to("/login")
}

// Handler GET /login — muestra el formulario de login.
pub async fn login_get() -> axum::response::Html<String> {
    axum::response::Html("<h1>Login</h1>".to_string())
}

// Handler POST /login — procesa el login.
pub async fn logout_post() -> axum::response::Redirect {
    axum::response::Redirect::to("/login")
}