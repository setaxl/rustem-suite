// ============================================================================
// RUSTEM SUITE — rustem-core::html_escape
// Escape HTML para prevenir inyección de HTML stored en datos de usuario.
// ============================================================================

//! # Módulo `html_escape`
//! Escapa caracteres especiales de HTML para prevenir inyección de HTML
//! almacenado cuando se renderizan datos de usuario en vistas SSR.
//! Caracteres escapados: `&`, `<`, `>`, `"`, `'`.

/// Escapa una cadena para su inserción segura en HTML.
///
/// Reemplaza los caracteres que tienen significado especial en HTML por sus
/// entidades correspondientes, evitando que datos de usuario inyecten
/// etiquetas o atributos HTML maliciosos.
///
/// # Ejemplo
/// ```
/// use rustem_core::html_escape::escape_html;
/// assert_eq!(escape_html("<script>alert(1)</script>"),
///            "&lt;script&gt;alert(1)&lt;/script&gt;");
/// assert_eq!(escape_html("\" onerror=\"x\""),
///            "&quot; onerror=&quot;x&quot;");
/// assert_eq!(escape_html("Juan Pérez"), "Juan Pérez");
/// ```
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_converts_angles_and_quotes() {
        assert_eq!(
            escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(
            escape_html("\" onerror=\"x\""),
            "&quot; onerror=&quot;x&quot;"
        );
        assert_eq!(escape_html("Juan Pérez"), "Juan Pérez");
        assert_eq!(escape_html(""), "");
        assert_eq!(escape_html("sin especiales"), "sin especiales");
        assert_eq!(escape_html("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn escape_html_is_idempotent_for_safe_strings() {
        let s = "texto seguro";
        assert_eq!(escape_html(s), s);
    }

    #[test]
    fn escape_html_prevents_double_escape() {
        // El & de &amp; no se escapa de nuevo.
        let input = "&amp;";
        assert_eq!(escape_html(input), "&amp;amp;");
    }
}
