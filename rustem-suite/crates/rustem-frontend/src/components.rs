// ============================================================================
// RUSTEM SUITE — rustem-frontend::components
// Componentes HTML reutilizables (SSR puro, Leptos).
// ============================================================================

//! # Módulo `components`
//! Componentes de interfaz renderizados como HTML estático.
//! Proporciona tanto funciones helper con `String` como un componente
//! [`Layout`] de Leptos en modo SSR puro (sin hidratación WASM).

use leptos::*;

/// Layout maestro con navbar y contenedor principal, renderizado con Leptos SSR.
///
/// Este componente no incluye hidratación del lado del cliente: el HTML
/// generado es el producto final de entrega (CERO JavaScript en el navegador).
#[component]
pub fn Layout(
    /// Título de la página.
    title: &'static str,
    /// Cuerpo HTML pre-renderizado como String (sin escapar en Leptos).
    #[prop(optional)]
    body: Option<String>,
) -> impl IntoView {
    let body_html = body.unwrap_or_default();
    view! {
        <html lang="es">
            <head>
                <meta charset="UTF-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <title>{title.to_string()}</title>
                <link rel="stylesheet" href="/static/css/base.css"/>
            </head>
            <body>
                <nav class="navbar">
                    <div class="navbar-inner">
                        <a href="/" class="brand">Rustem Suite</a>
                    </div>
                </nav>
                <main class="page container">
                    {body_html}
                </main>
            </body>
        </html>
    }
}

/// Renderiza un componente Leptos a una cadena HTML usando SSR puro.
///
/// Útil para integrar componentes Leptos en handlers de Axum que devuelven
/// `Html<String>`. No ejecuta JavaScript en el cliente.
pub fn render_ssr<F, V>(f: F) -> String
where
    F: FnOnce() -> V,
    V: IntoView,
    F: 'static,
{
    leptos::ssr::render_to_string(f).to_string()
}

/// Layout maestro con navbar y contenedor principal (versión String helper).
pub fn layout(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
        </div>
    </nav>
    <main class="page container">
        {body}
    </main>
</body>
</html>"#
    )
}

/// Tarjeta de contenido genérica.
pub fn card(title: &str, content: &str) -> String {
    format!(
        r#"<div class="card">
            <h2>{title}</h2>
            {content}
        </div>"#
    )
}

/// Mensaje flash (éxito/error/info).
pub fn flash(message: &str, css_class: &str) -> String {
    format!(r#"<div class="flash {css_class}">{message}</div>"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_no_js() {
        let html = layout("Test", "<p>contenido</p>");
        assert!(!html.contains("<script"));
        assert!(html.contains("Test"));
        assert!(html.contains("<p>contenido</p>"));
    }

    #[test]
    fn card_renders() {
        let html = card("Mi tarjeta", "<p>contenido de tarjeta</p>");
        assert!(html.contains("Mi tarjeta"));
        assert!(html.contains("class=\"card\""));
    }

    #[test]
    fn flash_renders() {
        let html = flash("Operación exitosa", "flash-success");
        assert!(html.contains("Operación exitosa"));
        assert!(html.contains("flash-success"));
    }

    #[test]
    fn render_ssr_produces_html() {
        let html = render_ssr(|| {
            view! { <p>Hola SSR</p> }
        });
        assert!(html.contains("Hola SSR"));
        assert!(!html.contains("<script"));
    }
}
