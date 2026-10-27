// ============================================================================
// RUSTEM SUITE — rustem-frontend::views
// Vistas Leptos SSR puro (cero JavaScript en cliente).
// ============================================================================

//! # Módulo `views`
//! Vistas renderizadas del lado del servidor con HTML nativo.
//! CERO JavaScript: solo formularios PRG y CSS puro.

use rustem_auth::handlers::FlashMessage;
use rustem_core::escape_html;
use rustem_sales::CustomerViewData;

/// Renderiza un mensaje flash como HTML (si hay alguno).
fn render_flash(flash: Option<&FlashMessage>) -> String {
    match flash {
        Some(f) => format!(
            r#"<div class="flash {}">{}</div>"#,
            f.css_class(),
            match f {
                FlashMessage::Success(m) => m,
                FlashMessage::Error(m) => m,
                FlashMessage::Info(m) => m,
            }
        ),
        None => String::new(),
    }
}

/// Vista de la página de login (HTML nativo).
pub fn login_view(flash: Option<&FlashMessage>) -> String {
    let flash_html = render_flash(flash);
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rustem Suite - Iniciar Sesión</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <div class="auth-page">
        <div class="auth-card card">
            <h1 class="page-title">Rustem Suite</h1>
            <p class="page-subtitle">ERP Venezolano - Inicio de Sesión</p>
            {flash_html}
            <form method="post" action="/login" class="form">
                <div class="field">
                    <label class="label" for="email">Correo electrónico</label>
                    <input type="email" id="email" name="email" class="input" required>
                </div>
                <div class="field">
                    <label class="label" for="password">Contraseña</label>
                    <input type="password" id="password" name="password" class="input" required>
                </div>
                <button type="submit" class="button button-primary">Ingresar</button>
            </form>
            <p class="auth-footer">¿No tiene cuenta? <a href="/registro">Regístrese</a></p>
        </div>
    </div>
</body>
</html>"#,
    )
}

/// Vista de la página de registro (HTML nativo).
pub fn register_view(flash: Option<&FlashMessage>) -> String {
    let flash_html = render_flash(flash);
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rustem Suite - Registro</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <div class="auth-page">
        <div class="auth-card card">
            <h1 class="page-title">Rustem Suite</h1>
            <p class="page-subtitle">ERP Venezolano - Crear Cuenta</p>
            {flash_html}
            <form method="post" action="/registro" class="form">
                <div class="field">
                    <label class="label" for="email">Correo electrónico</label>
                    <input type="email" id="email" name="email" class="input" required>
                </div>
                <div class="field">
                    <label class="label" for="password">Contraseña</label>
                    <input type="password" id="password" name="password" class="input" required>
                </div>
                <div class="field">
                    <label class="label" for="full_name">Nombre completo</label>
                    <input type="text" id="full_name" name="full_name" class="input" required>
                </div>
                <div class="field">
                    <label class="label" for="company_name">Nombre de la empresa</label>
                    <input type="text" id="company_name" name="company_name" class="input" required>
                </div>
                <div class="field">
                    <label class="label" for="company_rif">RIF de la empresa</label>
                    <input type="text" id="company_rif" name="company_rif" class="input" placeholder="J-12345678-9" required>
                </div>
                <button type="submit" class="button button-primary">Registrarse</button>
            </form>
            <p class="auth-footer">¿Ya tiene cuenta? <a href="/login">Inicie sesión</a></p>
        </div>
    </div>
</body>
</html>"#,
    )
}

/// Vista de dashboard principal.
pub fn dashboard_view(user_name: &str, company_name: &str, flash: Option<&FlashMessage>) -> String {
    let flash_html = render_flash(flash);
    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rustem Suite - Panel</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/almacenes">Almacenes</a>
                <a href="/productos">Productos</a>
                <a href="/kardex">Kardex</a>
                <a href="/ventas">Ventas</a>
                <a href="/contabilidad">Contabilidad</a>
                <a href="/admin">Administración</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        {flash_html}
        <h1 class="page-title">Panel de Control</h1>
        <p class="page-subtitle">Bienvenido, {user_name}. Empresa: {company_name}</p>
        <div class="card stack">
            <h2>Módulos disponibles</h2>
            <a href="/almacenes" class="button button-primary">Gestión de Almacenes</a>
            <a href="/productos" class="button button-primary">Gestión de Productos</a>
            <a href="/kardex" class="button button-primary">Kardex de Inventario</a>
            <a href="/ventas" class="button button-primary">Ventas y Facturación</a>
            <a href="/contabilidad" class="button button-primary">Contabilidad VEN-NIF</a>
            <a href="/admin" class="button button-secondary">Administración</a>
        </div>
    </main>
</body>
</html>"#,
        user_name = escape_html(user_name),
        company_name = escape_html(company_name)
    )
}

/// Vista de error 404.
pub fn not_found_view() -> String {
    String::from(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <title>Rustem Suite - No encontrado</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">404 - Página no encontrada</h1>
        <a href="/" class="button button-primary">Volver al inicio</a>
    </main>
</body>
</html>"#,
    )
}

/// Datos simplificados para renderizar un almacén en la vista.
#[derive(Debug, Clone)]
pub struct WarehouseViewData {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub is_active: bool,
}

/// Vista de lista de almacenes.
pub fn warehouses_list_view(warehouses: &[WarehouseViewData]) -> String {
    let mut rows = String::new();
    for w in warehouses {
        let badge = if w.is_active {
            "<span class=\"badge badge-success\">Activo</span>"
        } else {
            "<span class=\"badge badge-danger\">Inactivo</span>"
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{name}</td>
                <td>{address}</td>
                <td>{phone}</td>
                <td>{badge}</td>
                <td>
                    <a href="/almacenes/{id}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Editar</a>
                </td>
            </tr>"#,
            id = escape_html(&w.id),
            name = escape_html(&w.name),
            address = escape_html(w.address.as_deref().unwrap_or("-")),
            phone = escape_html(w.phone.as_deref().unwrap_or("-")),
            badge = badge
        ));
    }
    format!(
        r#"{header}<div class="card stack">
            <h2>Almacenes</h2>
            <a href="/almacenes/nuevo" class="button button-primary">+ Nuevo Almacén</a>
            <table class="table">
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Nombre</th>
                        <th>Dirección</th>
                        <th>Teléfono</th>
                        <th>Estado</th>
                        <th>Acciones</th>
                    </tr>
                </thead>
                <tbody>
                    {rows}
                </tbody>
            </table>
        </div>{footer}"#,
        header = crate_views_layout_header("Almacenes"),
        rows = rows,
        footer = crate_views_layout_footer()
    )
}

/// Vista de formulario de almacén (crear o editar).
pub fn warehouse_form_view(warehouse: Option<&WarehouseViewData>) -> String {
    let (action, _id_value, name_value, address_value, phone_value, title, checked) =
        match warehouse {
            Some(w) => (
                format!("/almacenes/{}/actualizar", w.id),
                w.id.clone(),
                escape_html(&w.name),
                escape_html(w.address.as_deref().unwrap_or_default()),
                escape_html(w.phone.as_deref().unwrap_or_default()),
                format!("Editar Almacén: {}", escape_html(&w.name)),
                if w.is_active { " checked" } else { "" }.to_string(),
            ),
            None => (
                "/almacenes/crear".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "Nuevo Almacén".to_string(),
                String::new(),
            ),
        };
    format!(
        r#"{header}<div class="card stack">
            <h2>{title}</h2>
            <form method="post" action="{action}" class="form">
                <div class="field">
                    <label class="label" for="name">Nombre del almacén</label>
                    <input type="text" id="name" name="name" class="input" value="{name_value}" required>
                </div>
                <div class="field">
                    <label class="label" for="address">Dirección</label>
                    <input type="text" id="address" name="address" class="input" value="{address_value}">
                </div>
                <div class="field">
                    <label class="label" for="phone">Teléfono</label>
                    <input type="tel" id="phone" name="phone" class="input" value="{phone_value}">
                </div>
                <div class="field">
                    <label class="label" for="is_active">Activo</label>
                    <input type="checkbox" id="is_active" name="is_active" value="1"{checked}>
                </div>
                <button type="submit" class="button button-primary">Guardar</button>
                <a href="/almacenes" class="button button-secondary">Cancelar</a>
            </form>
        </div>{footer}"#,
        header = crate_views_layout_header("Almacén"),
        title = title,
        action = action,
        name_value = name_value,
        address_value = address_value,
        phone_value = phone_value,
        checked = checked,
        footer = crate_views_layout_footer()
    )
}

/// Datos simplificados para renderizar un producto en la vista.
#[derive(Debug, Clone)]
pub struct ProductViewData {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub vat_rate: u8,
    pub unit: String,
    pub category: Option<String>,
}

/// Vista de lista de productos.
pub fn products_list_view(products: &[ProductViewData]) -> String {
    let mut rows = String::new();
    for p in products {
        rows.push_str(&format!(
            r#"<tr>
                <td>{id}</td>
                <td>{sku}</td>
                <td>{name}</td>
                <td>{vat_rate} %</td>
                <td>{unit}</td>
                <td>{category}</td>
                <td>
                    <a href="/productos/{id}/editar" class="button button-secondary" style="padding:0.25rem 0.5rem; font-size:0.875rem;">Editar</a>
                </td>
            </tr>"#,
            id = escape_html(&p.id),
            sku = escape_html(&p.sku),
            name = escape_html(&p.name),
            vat_rate = p.vat_rate,
            unit = escape_html(&p.unit),
            category = escape_html(p.category.as_deref().unwrap_or("-")),
        ));
    }
    format!(
        r#"{header}<div class="card stack">
            <h2>Productos</h2>
            <a href="/productos/nuevo" class="button button-primary">+ Nuevo Producto</a>
            <table class="table">
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>SKU</th>
                        <th>Nombre</th>
                        <th>IVA</th>
                        <th>Unidad</th>
                        <th>Categoría</th>
                        <th>Acciones</th>
                    </tr>
                </thead>
                <tbody>
                    {rows}
                </tbody>
            </table>
        </div>{footer}"#,
        header = crate_views_layout_header("Productos"),
        rows = rows,
        footer = crate_views_layout_footer()
    )
}

/// Vista de formulario de producto (crear o editar).
pub fn product_form_view(product: Option<&ProductViewData>) -> String {
    let (action_value, sku_value, name_value, vat_value, unit_value, category_value, title) =
        match product {
            Some(p) => (
                format!("/productos/{}/actualizar", p.id),
                escape_html(&p.sku),
                escape_html(&p.name),
                p.vat_rate,
                escape_html(&p.unit),
                escape_html(p.category.as_deref().unwrap_or_default()),
                format!("Editar Producto: {}", escape_html(&p.name)),
            ),
            None => (
                "/productos/crear".to_string(),
                String::new(),
                String::new(),
                16u8,
                "und".to_string(),
                String::new(),
                "Nuevo Producto".to_string(),
            ),
        };
    let vat_options = [
        ("16", "General (16%)"),
        ("8", "Reducida (8%)"),
        ("0", "Exento (0%)"),
    ];
    let mut vat_html = String::new();
    for (val, label) in vat_options {
        let selected = if vat_value.to_string() == val {
            " selected"
        } else {
            ""
        };
        vat_html.push_str(&format!(
            "<option value=\"{val}\"{selected}>{label}</option>"
        ));
    }
    format!(
        r#"{header}<div class="card stack">
            <h2>{title}</h2>
            <form method="post" action="{action}" class="form">
                <div class="field">
                    <label class="label" for="sku">SKU</label>
                    <input type="text" id="sku" name="sku" class="input" value="{sku_value}" required>
                </div>
                <div class="field">
                    <label class="label" for="name">Nombre</label>
                    <input type="text" id="name" name="name" class="input" value="{name_value}" required>
                </div>
                <div class="field">
                    <label class="label" for="vat_rate">Alícuota IVA</label>
                    <select id="vat_rate" name="vat_rate" class="input">
                        {vat_html}
                    </select>
                </div>
                <div class="field">
                    <label class="label" for="unit">Unidad de medida</label>
                    <input type="text" id="unit" name="unit" class="input" value="{unit_value}">
                </div>
                <div class="field">
                    <label class="label" for="category">Categoría</label>
                    <input type="text" id="category" name="category" class="input" value="{category_value}">
                </div>
                <button type="submit" class="button button-primary">Guardar</button>
                <a href="/productos" class="button button-secondary">Cancelar</a>
            </form>
        </div>{footer}"#,
        header = crate_views_layout_header("Producto"),
        title = title,
        action = action_value,
        sku_value = sku_value,
        name_value = name_value,
        vat_html = vat_html,
        unit_value = unit_value,
        category_value = category_value,
        footer = crate_views_layout_footer()
    )
}

/// Datos de kardex para la vista.
#[derive(Debug, Clone)]
pub struct KardexViewData {
    pub created_at: String,
    pub movement_type_label: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_cost_cents: i64,
    pub reference: Option<String>,
}

/// Vista de kardex (movimientos de stock).
pub fn kardex_view(movements: &[KardexViewData]) -> String {
    let mut rows = String::new();
    for m in movements {
        let sign = if m.movement_type_label == "ENTRADA" || m.movement_type_label == "AJUSTE" {
            "+"
        } else {
            "-"
        };
        rows.push_str(&format!(
            r#"<tr>
                <td>{created_at}</td>
                <td>{movement_type_label}</td>
                <td>{sign} {quantity}</td>
                <td>{unit_cost_cents}</td>
                <td>{reference}</td>
            </tr>"#,
            created_at = m.created_at,
            movement_type_label = m.movement_type_label,
            sign = sign,
            quantity = m.quantity,
            unit_cost_cents = m.unit_cost_cents,
            reference = m.reference.as_deref().unwrap_or("-"),
        ));
    }
    format!(
        r#"{header}<div class="card stack">
            <h2>Kardex - Movimientos de Stock</h2>
            <table class="table">
                <thead>
                    <tr>
                        <th>Fecha</th>
                        <th>Tipo</th>
                        <th>Cantidad</th>
                        <th>Costo Unit.</th>
                        <th>Referencia</th>
                    </tr>
                </thead>
                <tbody>
                    {rows}
                </tbody>
            </table>
        </div>{footer}"#,
        header = crate_views_layout_header("Kardex"),
        rows = rows,
        footer = crate_views_layout_footer()
    )
}

/// Encabezado de layout reutilizado por las vistas de inventario.
fn crate_views_layout_header(title: &str) -> String {
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
            <div class="nav-links">
                <a href="/almacenes">Almacenes</a>
                <a href="/productos">Productos</a>
                <a href="/kardex">Kardex</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">{title}</h1>"#
    )
}

/// Pie de layout reutilizado por las vistas de inventario.
fn crate_views_layout_footer() -> String {
    String::from(
        r#"    </main>
</body>
</html>"#,
    )
}

/// Vista de formulario de cliente (crear o editar).
pub fn customer_form_view(customer: Option<&CustomerViewData>) -> String {
    let (
        action,
        _id_value,
        rif_value,
        name_value,
        address_value,
        phone_value,
        email_value,
        title,
        vat_selected,
    ) = match customer {
        Some(c) => (
            format!("/clientes/{}/actualizar", c.id),
            c.id.clone(),
            escape_html(&c.rif),
            escape_html(&c.name),
            escape_html(c.address.as_deref().unwrap_or_default()),
            escape_html(c.phone.as_deref().unwrap_or_default()),
            escape_html(c.email.as_deref().unwrap_or_default()),
            format!("Editar Cliente: {}", escape_html(&c.name)),
            c.vat_condition.clone(),
        ),
        None => (
            "/clientes/crear".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "Nuevo Cliente".to_string(),
            "ordinario".to_string(),
        ),
    };

    let conditions = [
        ("ordinario", "Contribuyente Ordinario"),
        ("formal", "Contribuyente Formal"),
        ("espe", "Sujeto Pasivo Especial (SPE)"),
        ("no_sujeto", "No Sujeto / Exento"),
    ];
    let mut condition_html = String::new();
    for (val, label) in &conditions {
        let selected = if vat_selected == *val {
            " selected"
        } else {
            ""
        };
        condition_html.push_str(&format!(
            "<option value=\"{val}\"{selected}>{label}</option>"
        ));
    }

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
            <div class="nav-links">
                <a href="/clientes">Clientes</a>
                <a href="/facturas">Facturas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">{title}</h1>
        <form method="post" action="{action}" class="form">
            <div class="field">
                <label class="label" for="rif">RIF</label>
                <input type="text" id="rif" name="rif" class="input" value="{rif_value}" placeholder="J-12345678-9" required>
            </div>
            <div class="field">
                <label class="label" for="name">Nombre</label>
                <input type="text" id="name" name="name" class="input" value="{name_value}" required>
            </div>
            <div class="field">
                <label class="label" for="vat_condition">Condición IVA</label>
                <select id="vat_condition" name="vat_condition" class="input">
                    {condition_html}
                </select>
            </div>
            <div class="field">
                <label class="label" for="address">Dirección</label>
                <input type="text" id="address" name="address" class="input" value="{address_value}">
            </div>
            <div class="field">
                <label class="label" for="phone">Teléfono</label>
                <input type="tel" id="phone" name="phone" class="input" value="{phone_value}">
            </div>
            <div class="field">
                <label class="label" for="email">Email</label>
                <input type="email" id="email" name="email" class="input" value="{email_value}">
            </div>
            <button type="submit" class="button button-primary">Guardar</button>
            <a href="/clientes" class="button button-secondary">Cancelar</a>
        </form>
    </main>
</body>
</html>"#,
    )
}

/// Nodo de cuenta contable para renderizar árbol jerárquico.
#[derive(Debug, Clone)]
pub struct AccountNode {
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub level: u8,
    pub parent_code: Option<String>,
    pub is_active: bool,
    pub children: Vec<AccountNode>,
}

/// Vista de árbol de cuentas contables usando `<details>/<summary>` (HTML nativo, cero JS).
pub fn cuentas_arbol_view(accounts: &[AccountNode]) -> String {
    let mut html = String::new();

    // Filtrar solo cuentas raíz (sin parent_code o parent_code vacío).
    let root_accounts: Vec<&AccountNode> = accounts
        .iter()
        .filter(|a| a.parent_code.is_none() || a.parent_code.as_deref() == Some(""))
        .collect();

    for account in root_accounts {
        html.push_str(&render_account_node(account, 0));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Árbol de Cuentas - Rustem Suite</title>
    <link rel="stylesheet" href="/static/css/base.css">
</head>
<body>
    <nav class="navbar">
        <div class="navbar-inner">
            <a href="/" class="brand">Rustem Suite</a>
            <div class="nav-links">
                <a href="/contabilidad/cuentas">Plan de Cuentas</a>
                <a href="/contabilidad/cuentas/arbol">Árbol de Cuentas</a>
                <a href="/dashboard">Panel</a>
            </div>
        </div>
    </nav>
    <main class="page container">
        <h1 class="page-title">Árbol de Cuentas Contables</h1>
        <div class="card stack">
            {html}
        </div>
    </main>
</body>
</html>"#,
    )
}

/// Renderiza un nodo de cuenta y sus hijos recursivamente usando <details>/<summary>.
fn render_account_node(account: &AccountNode, depth: u8) -> String {
    let code = escape_html(&account.code);
    let name = escape_html(&account.name);
    let account_type = escape_html(&account.account_type);
    let badge = if account.is_active {
        "<span class=\"badge badge-success\">Activo</span>"
    } else {
        "<span class=\"badge badge-danger\">Inactivo</span>"
    };

    let children_html = if account.children.is_empty() {
        String::new()
    } else {
        let mut children = String::new();
        for child in &account.children {
            children.push_str(&render_account_node(child, depth + 1));
        }
        format!("<div class=\"card stack\" style=\"margin-left: 2rem;\">{children}</div>")
    };

    format!(
        r#"<details class="account-node level-{depth}">
            <summary>
                <span class="account-code">{code}</span>
                <span class="account-name">{name}</span>
                <span class="account-type">{account_type}</span>
                <span class="account-badge">{badge}</span>
            </summary>
            {children_html}
        </details>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_view_has_no_js() {
        let html = login_view(None);
        assert!(!html.contains("<script"));
        assert!(!html.contains("onclick="));
        assert!(!html.contains("onchange="));
        assert!(html.contains("Rustem Suite"));
        assert!(html.contains("/login"));
    }

    #[test]
    fn login_view_renders_flash() {
        let html = login_view(Some(&FlashMessage::error(
            "credenciales inválidas".to_string(),
        )));
        assert!(html.contains("flash-error"));
        assert!(html.contains("credenciales inválidas"));
    }

    #[test]
    fn dashboard_view_has_no_js() {
        let html = dashboard_view("Juan", "Empresa Demo", None);
        assert!(!html.contains("<script"));
        assert!(!html.contains("onclick="));
        assert!(!html.contains("onchange="));
        assert!(!html.contains("onsubmit="));
        assert!(!html.contains("onload="));
        assert!(!html.contains("onmouse"));
        assert!(html.contains("Juan"));
        assert!(html.contains("Empresa Demo"));
    }

    #[test]
    fn views_contain_css_link() {
        assert!(login_view(None).contains("/static/css/base.css"));
        assert!(dashboard_view("x", "y", None).contains("/static/css/base.css"));
    }

    #[test]
    fn warehouses_list_view_has_no_js() {
        let html = warehouses_list_view(&[]);
        assert!(!html.contains("<script"));
        assert!(!html.contains("onclick="));
        assert!(html.contains("Almacenes"));
    }

    #[test]
    fn warehouse_form_view_has_no_js() {
        let html = warehouse_form_view(None);
        assert!(!html.contains("<script"));
        assert!(html.contains("Nuevo Almacén"));
        assert!(html.contains("method=\"post\""));
    }

    #[test]
    fn products_list_view_has_no_js() {
        let html = products_list_view(&[]);
        assert!(!html.contains("<script"));
        assert!(html.contains("Productos"));
    }

    #[test]
    fn product_form_view_has_no_js() {
        let html = product_form_view(None);
        assert!(!html.contains("<script"));
        assert!(html.contains("Nuevo Producto"));
        assert!(html.contains("value=\"16\""));
    }

    #[test]
    fn kardex_view_has_no_js() {
        let html = kardex_view(&[]);
        assert!(!html.contains("<script"));
        assert!(html.contains("Kardex"));
    }

    #[test]
    fn warehouse_form_view_with_data() {
        let w = WarehouseViewData {
            id: "w1".to_string(),
            name: "Almacén Test".to_string(),
            address: Some("Calle 1".to_string()),
            phone: Some("0212-1234567".to_string()),
            is_active: true,
        };
        let html = warehouse_form_view(Some(&w));
        assert!(html.contains("Almacén Test"));
        assert!(html.contains("Calle 1"));
        assert!(html.contains("checked"));
    }

    #[test]
    fn product_form_view_with_data() {
        let p = ProductViewData {
            id: "p1".to_string(),
            sku: "SKU-001".to_string(),
            name: "Producto Test".to_string(),
            vat_rate: 8,
            unit: "und".to_string(),
            category: Some("Electrónica".to_string()),
        };
        let html = product_form_view(Some(&p));
        assert!(html.contains("Producto Test"));
        assert!(html.contains("value=\"8\""));
        assert!(html.contains("selected"));
    }
}
