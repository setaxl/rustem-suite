// Handler POST /logout — proceso de logout.
pub async fn logout_post() -> axum::response::Redirect {
    axum::response::Redirect::to("/login")
}