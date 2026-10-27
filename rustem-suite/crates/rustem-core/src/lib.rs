// ============================================================================
// RUSTEM SUITE — Crate rustem-core
// Tipos de dominio compartidos, VEN-NIF Money, RIF y errores del dominio.
// Todos los comentarios y documentación en español.
// ============================================================================

//! # Crate `rustem-core`
//! Contiene los tipos de dominio compartidos por todos los módulos de
//! Rustem Suite:
//! - [`Money`]: importe monetario exacto en centavos (`i64`) con código ISO
//!   4217 (`VES`, `USD`). Prohibido usar `f32`/`f64` para dinero.
//! - [`Currency`]: códigos de moneda soportados (Bolívar y Dólar).
//! - [`Rif`]: Registro de Información Fiscal venezolano con dígito
//!   verificador.
//! - [`RustemError`]: tipo de error del dominio (`thiserror`).
//! - [`DomainResult`]: alias de `Result` tipificado con `RustemError`.

pub mod currency;
pub mod errors;
pub mod html_escape;
pub mod money;
pub mod rif;

pub use currency::Currency;
pub use errors::{DomainResult, RustemError};
pub use html_escape::escape_html;
pub use money::Money;
pub use rif::Rif;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_smoke_test() {
        // Prueba de humo: los tipos del núcleo se instancian y se comparan.
        let m1 = Money::new(1000, Currency::Ves).unwrap();
        let m2 = Money::new(1000, Currency::Ves).unwrap();
        assert_eq!(m1, m2);

        let rif = Rif::new("V-12345678-9").unwrap();
        assert_eq!(rif.as_str(), "V-12345678-9");
    }
}
