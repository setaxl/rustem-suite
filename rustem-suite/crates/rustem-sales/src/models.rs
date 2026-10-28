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

    #[test]
    fn vat_condition_all_variants() {
        let c_ord = Customer::new("c1", "V-12345678-9", "C1", VatCondition::Ordinario);
        let c_for = Customer::new("c2", "V-12345679-0", "C2", VatCondition::Formal);
        let c_espe = Customer::new("c3", "V-12345680-1", "C3", VatCondition::Espe);
        let c_no = Customer::new("c4", "V-12345681-2", "C4", VatCondition::NoSujeto);
        assert_eq!(c_ord.vat_condition_str(), "ordinario");
        assert_eq!(c_for.vat_condition_str(), "formal");
        assert_eq!(c_espe.vat_condition_str(), "espe");
        assert_eq!(c_no.vat_condition_str(), "no_sujeto");
    }

        #[test]
    fn document_state_all_variants() {
        let quote_borrador = Quote {
            id: "q1".to_string(),
            customer_id: "c1".to_string(),
            state: DocumentState::Borrador,
            subtotal_cents: 0,
            iva_cents: 0,
            igtf_cents: 0,
            total_cents: 0,
            currency: "VES".to_string(),
        };
        assert_eq!(quote_borrador.state, DocumentState::Borrador);

        let quote_emitido = Quote {
            id: "q2".to_string(),
            customer_id: "c1".to_string(),
            state: DocumentState::Emitido,
            subtotal_cents: 0,
            iva_cents: 0,
            igtf_cents: 0,
            total_cents: 0,
            currency: "VES".to_string(),
        };
        assert_eq!(quote_emitido.state, DocumentState::Emitido);

        let quote_anulado = Quote {
            id: "q3".to_string(),
            customer_id: "c1".to_string(),
            state: DocumentState::Anulado,
            subtotal_cents: 0,
            iva_cents: 0,
            igtf_cents: 0,
            total_cents: 0,
            currency: "VES".to_string(),
        };
        assert_eq!(quote_anulado.state, DocumentState::Anulado);

        let order_pagado = Order {
            id: "o1".to_string(),
            quote_id: None,
            customer_id: "c1".to_string(),
            state: DocumentState::Pagado,
            total_cents: 5000,
        };
        assert_eq!(order_pagado.state, DocumentState::Pagado);
    }

    #[test]
    fn quote_field_accessors() {
        let quote = Quote {
            id: "q-test".to_string(),
            customer_id: "c1".to_string(),
            state: DocumentState::Emitido,
            subtotal_cents: 10000,
            iva_cents: 1600,
            igtf_cents: 0,
            total_cents: 11600,
            currency: "VES".to_string(),
        };
        assert_eq!(quote.id, "q-test");
        assert_eq!(quote.customer_id, "c1");
        assert_eq!(quote.total_cents, 11600);
        assert_eq!(quote.currency, "VES");
    }

    #[test]
    fn order_field_accessors() {
        let order = Order {
            id: "o1".to_string(),
            quote_id: Some("q1".to_string()),
            customer_id: "c1".to_string(),
            state: DocumentState::Pagado,
            total_cents: 5000,
        };
        assert_eq!(order.id, "o1");
        assert_eq!(order.customer_id, "c1");
        assert_eq!(order.total_cents, 5000);
        assert_eq!(order.quote_id, Some("q1".to_string()));
    }

    #[test]
    fn customer_view_data_creation() {
        let cvd = CustomerViewData {
            id: "c1".to_string(),
            rif: "V-12345678-9".to_string(),
            name: "Cliente Test".to_string(),
            vat_condition: "ordinario".to_string(),
            address: Some("Calle 1".to_string()),
            phone: Some("1234567".to_string()),
            email: Some("test@test.com".to_string()),
        };
        assert_eq!(cvd.id, "c1");
        assert_eq!(cvd.rif, "V-12345678-9");
        assert_eq!(cvd.name, "Cliente Test");
    }
}
