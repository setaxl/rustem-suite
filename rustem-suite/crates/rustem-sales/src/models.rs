// ============================================================================
// RUSTEM SUITE — rustem-sales::models
// Modelos de ventas: clientes, cotizaciones, pedidos, facturas.
// ============================================================================

use serde::{Deserialize, Serialize};

/// Condición IVA del cliente ante el SENIAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VatCondition {
    Ordinario, // Contribuyente ordinario
    Formal,    // Contribuyente formal
    Espe,      // Sujeto Pasivo Especial (SPE)
    NoSujeto,  // No sujeto / exento
}

/// Cliente con RIF y condición fiscal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub rif: String,
    pub name: String,
    pub vat_condition: VatCondition,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

impl Customer {
    pub fn new(
        id: impl Into<String>,
        rif: impl Into<String>,
        name: impl Into<String>,
        vat_condition: VatCondition,
    ) -> Self {
        Self {
            id: id.into(),
            rif: rif.into(),
            name: name.into(),
            vat_condition,
            address: None,
            phone: None,
            email: None,
        }
    }

    /// Devuelve la condición IVA como cadena para formularios.
    pub fn vat_condition_str(&self) -> &'static str {
        match self.vat_condition {
            VatCondition::Ordinario => "ordinario",
            VatCondition::Formal => "formal",
            VatCondition::Espe => "espe",
            VatCondition::NoSujeto => "no_sujeto",
        }
    }
}

/// Datos simplificados para renderizar un cliente en la vista.
#[derive(Debug, Clone)]
pub struct CustomerViewData {
    pub id: String,
    pub rif: String,
    pub name: String,
    pub vat_condition: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// Estado de una cotización/pedido/factura.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentState {
    Borrador,
    Emitido,
    Anulado,
    Pagado,
}

/// Cotización de venta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub id: String,
    pub customer_id: String,
    pub state: DocumentState,
    pub subtotal_cents: i64,
    pub iva_cents: i64,
    pub igtf_cents: i64,
    pub total_cents: i64,
    pub currency: String,
}

/// Pedido de venta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub quote_id: Option<String>,
    pub customer_id: String,
    pub state: DocumentState,
    pub total_cents: i64,
}

/// Factura fiscal (SENIAT).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub order_id: Option<String>,
    pub customer_id: String,
    pub series: String,         // ej. "001"
    pub number: u32,            // correlativo dentro de la serie
    pub control_number: String, // Número de control SENIAT
    pub state: DocumentState,
    pub subtotal_cents: i64,
    pub iva_cents: i64,
    pub igtf_cents: i64,
    pub total_cents: i64,
    pub currency: String,
    pub exchange_rate: Option<String>, // Tasa BCV aplicada
}

#[cfg(test)]
mod tests {
    use super::*;

    use VatCondition::Ordinario as VatConditionOrdinario;

    #[test]
    fn customer_creation() {
        let c = Customer::new("c1", "V-12345678-9", "Cliente Demo", VatConditionOrdinario);
        assert_eq!(c.name, "Cliente Demo");
        assert_eq!(c.rif, "V-12345678-9");
    }

    #[test]
    fn invoice_total() {
        let inv = Invoice {
            id: "f1".to_string(),
            order_id: None,
            customer_id: "c1".to_string(),
            series: "001".to_string(),
            number: 1,
            control_number: "001-000001".to_string(),
            state: DocumentState::Emitido,
            subtotal_cents: 10000,
            iva_cents: 1600,
            igtf_cents: 0,
            total_cents: 11600,
            currency: "VES".to_string(),
            exchange_rate: None,
        };
        assert_eq!(inv.total_cents, 11600);
    }
}
