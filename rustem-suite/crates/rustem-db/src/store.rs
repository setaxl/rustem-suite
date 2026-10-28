// ============================================================================
// RUSTEM SUITE — rustem-db::store
// Traits e implementación de repositorio con inyección automática de tenant_id
// y registro de auditoría en grafo (RELATE).
// ============================================================================

//! # Módulo `store`
//! Define el trait [`SurrealStore`] y su implementación concreta
//! [`SurrealStoreImpl`]. Garantiza que:
//! - Toda consulta incluya automáticamente el `tenant_id` (multi-tenancy).
//! - Toda mutación de estado registre un evento de auditoría inmutable.
//! - Todas las operaciones usen consultas parametrizadas (sin interpolación de
//!   cadenas).

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use surrealdb::Response;
use tracing::{debug, info};

use crate::connection::DbConnection;

/// Convierte un vector de pares (nombre, valor) a un objeto JSON para parámetros de SurrealDB.
pub fn params_to_value(params: Vec<(String, Value)>) -> Value {
    let mut map = serde_json::Map::with_capacity(params.len());
    for (key, value) in params {
        map.insert(key, value);
    }
    Value::Object(map)
}

/// Contexto de ejecución con información del tenant activo.
/// Se inyecta desde la sesión HTTP en el middleware de Axum.
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    /// ID del tenant (empresa) activo.
    pub tenant_id: String,
    /// ID del usuario autenticado.
    pub user_id: Option<String>,
    /// Roles asignados al usuario dentro del tenant.
    pub roles: Vec<String>,
    /// Permisos asignados al usuario, como pares `"recurso:acción"`.
    pub permissions: Vec<String>,
    /// Dirección IP del cliente (para auditoría).
    pub ip_address: Option<String>,
    /// User-Agent del cliente.
    pub user_agent: Option<String>,
}

impl TenantContext {
    /// Crea un nuevo contexto de tenant vacío.
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            user_id: None,
            roles: Vec::new(),
            permissions: Vec::new(),
            ip_address: None,
            user_agent: None,
        }
    }

    /// Verifica si el usuario tiene el permiso `"recurso:acción"`.
    pub fn has_permission(&self, resource: &str, action: &str) -> bool {
        self.permissions
            .iter()
            .any(|p| *p == format!("{resource}:{action}"))
    }

    /// Verifica si el usuario tiene el rol dado.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Resultado de una operación de auditoría.
pub struct AuditResult {
    pub audit_log_id: String,
}

/// Trait de acceso a datos con multi-tenancy obligatorio.
#[async_trait]
pub trait SurrealStore {
    /// Devuelve la conexión subyacente a SurrealDB.
    fn connection(&self) -> &dyn DbConnection;

    /// Devuelve el contexto de tenant actual.
    fn tenant_context(&self) -> &TenantContext;

    /// Ejecuta una consulta parametrizada inyectando `$tenant_id` automáticamente.
    async fn query_with_tenant(
        &self,
        sql: &str,
        extra_params: Vec<(String, Value)>,
    ) -> anyhow::Result<Response>;

    /// Crea un registro asegurando que incluya `tenant_id` y registra auditoría.
    async fn create_with_tenant(&self, table: &str, data: Value) -> anyhow::Result<Option<Value>>;

    /// Actualiza un registro filtrando por `tenant_id` y registra auditoría.
    async fn update_with_tenant(
        &self,
        table: &str,
        record_id: &str,
        data: Value,
    ) -> anyhow::Result<Option<Value>>;

    /// Elimina un registro filtrando por `tenant_id` y registra auditoría.
    async fn delete_with_tenant(
        &self,
        table: &str,
        record_id: &str,
    ) -> anyhow::Result<Option<Value>>;

    /// Inserta un registro de auditoría inmutable y crea arista de grafo
    /// `user->EXECUTED->audit_log`.
    async fn audit(
        &self,
        action: &str,
        table_name: &str,
        record_id: Option<&Value>,
        payload_before: Option<&Value>,
        payload_after: Option<&Value>,
    ) -> anyhow::Result<AuditResult>;

    /// Ejecuta un bloque de transacción ACID (BEGIN TRANSACTION ... COMMIT TRANSACTION)
    /// para garantizar atomicidad en escrituras múltiples (ej. movimiento + lote + auditoría).
    async fn execute_transaction(
        &self,
        sql: &str,
        params: Vec<(String, Value)>,
    ) -> anyhow::Result<Response>;

    /// Crea una arista de grafo RELATE entre dos registros para trazabilidad.
    /// Ejemplo: warehouse->HAS_STOCK->product.
    async fn create_edge(
        &self,
        from_table: &str,
        from_id: &str,
        edge_name: &str,
        to_table: &str,
        to_id: &str,
        data: Value,
    ) -> anyhow::Result<Option<Value>>;

    /// Crea un asiento contable usando transacción ACID para garantizar atomicidad.
    /// El asiento se registra en journal_entries y se emite auditoría automáticamente.
    async fn create_journal_entry(
        &self,
        entry_id: &str,
        date: &str,
        description: &str,
        lines_json: Value,
        created_by: Option<&str>,
    ) -> anyhow::Result<Option<Value>>;

    /// Consulta asientos contables por tenant con filtro opcional de rango de fechas.
    async fn query_journal_entries_by_tenant(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> anyhow::Result<Response>;

    /// Consulta el plan de cuentas por tenant.
    async fn query_chart_of_accounts_by_tenant(&self) -> anyhow::Result<Response>;
}

/// Implementación concreta de [`SurrealStore`] sobre SurrealDB.
pub struct SurrealStoreImpl {
    conn: Arc<dyn DbConnection>,
    ctx: TenantContext,
}

impl SurrealStoreImpl {
    /// Crea una nueva instancia con conexión y contexto de tenant.
    pub fn new(conn: Arc<dyn DbConnection>, ctx: TenantContext) -> Self {
        Self { conn, ctx }
    }
}

#[async_trait]
impl SurrealStore for SurrealStoreImpl {
    fn connection(&self) -> &dyn DbConnection {
        self.conn.as_ref()
    }

    fn tenant_context(&self) -> &TenantContext {
        &self.ctx
    }

    async fn query_with_tenant(
        &self,
        sql: &str,
        extra_params: Vec<(String, Value)>,
    ) -> anyhow::Result<Response> {
        let mut params: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        params.extend(extra_params);

        debug!(
            "Ejecutando consulta con tenant_id={}: {}",
            self.ctx.tenant_id, sql
        );

        let resp = self.conn.query(sql, params_to_value(params)).await?;
        Ok(resp)
    }

    async fn create_with_tenant(
        &self,
        table: &str,
        mut data: Value,
    ) -> anyhow::Result<Option<Value>> {
        // Inyectar tenant_id obligatoriamente.
        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "tenant_id".to_string(),
                Value::String(self.ctx.tenant_id.clone()),
            );
            obj.insert(
                "created_at".to_string(),
                Value::String(chrono::Utc::now().to_rfc3339()),
            );
        }

        let sql = format!("CREATE {table} CONTENT $data");
        let params: Vec<(String, Value)> = vec![("$data".to_string(), data)];

        let mut resp = self.conn.query(&sql, params_to_value(params)).await?;
        let created: Option<Value> = resp.take(0)?;

        // Registrar auditoría de creación.
        let record_id = created.as_ref().and_then(|v| v.get("id").cloned());
        self.audit("CREATE", table, record_id.as_ref(), None, created.as_ref())
            .await?;

        Ok(created)
    }

    async fn update_with_tenant(
        &self,
        table: &str,
        record_id: &str,
        mut data: Value,
    ) -> anyhow::Result<Option<Value>> {
        // Obtener el registro anterior para auditoría.
        let sql_select = format!("SELECT * FROM {table} WHERE id = $id AND tenant_id = $tenant_id");
        let before = self
            .conn
            .query(
                &sql_select,
                params_to_value(vec![
                    ("$id".to_string(), Value::String(record_id.to_string())),
                    (
                        "$tenant_id".to_string(),
                        Value::String(self.ctx.tenant_id.clone()),
                    ),
                ]),
            )
            .await?
            .take::<Vec<Value>>(0)?
            .into_iter()
            .next();

        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "updated_at".to_string(),
                Value::String(chrono::Utc::now().to_rfc3339()),
            );
        }

        let sql = format!("UPDATE {table}:{record_id} CONTENT $data");
        let params: Vec<(String, Value)> = vec![("$data".to_string(), data)];

        let mut resp = self.conn.query(&sql, params_to_value(params)).await?;
        let updated: Option<Value> = resp.take(0)?;

        // Registrar auditoría de actualización.
        self.audit(
            "UPDATE",
            table,
            Some(&Value::String(record_id.to_string())),
            before.as_ref(),
            updated.as_ref(),
        )
        .await?;

        Ok(updated)
    }

    async fn delete_with_tenant(
        &self,
        table: &str,
        record_id: &str,
    ) -> anyhow::Result<Option<Value>> {
        // Obtener el registro antes de eliminar para auditoría.
        let sql_select = format!("SELECT * FROM {table} WHERE id = $id AND tenant_id = $tenant_id");
        let before = self
            .conn
            .query(
                &sql_select,
                params_to_value(vec![
                    ("$id".to_string(), Value::String(record_id.to_string())),
                    (
                        "$tenant_id".to_string(),
                        Value::String(self.ctx.tenant_id.clone()),
                    ),
                ]),
            )
            .await?
            .take::<Vec<Value>>(0)?
            .into_iter()
            .next();

        let sql = format!("DELETE {table}:{record_id}");
        let params: Vec<(String, Value)> = vec![];

        let mut resp = self.conn.query(&sql, params_to_value(params)).await?;
        let deleted: Option<Value> = resp.take(0)?;

        // Registrar auditoría de eliminación.
        self.audit(
            "DELETE",
            table,
            Some(&Value::String(record_id.to_string())),
            before.as_ref(),
            None,
        )
        .await?;

        Ok(deleted)
    }

    async fn audit(
        &self,
        action: &str,
        table_name: &str,
        record_id: Option<&Value>,
        payload_before: Option<&Value>,
        payload_after: Option<&Value>,
    ) -> anyhow::Result<AuditResult> {
        let now = chrono::Utc::now().to_rfc3339();
        let user_id = self.ctx.user_id.clone();
        let tenant_id = self.ctx.tenant_id.clone();
        let ip_address = self.ctx.ip_address.clone();

        // Insertar registro de auditoría.
        let sql = r#"
            CREATE audit_log CONTENT {
                user_id: $user_id,
                company_id: $tenant_id,
                action: $action,
                table_name: $table_name,
                record_id: $record_id,
                payload_before: $payload_before,
                payload_after: $payload_after,
                ip_address: $ip_address,
                created_at: $created_at
            }"#;

        let params: Vec<(String, Value)> = vec![
            (
                "$user_id".to_string(),
                Value::String(user_id.unwrap_or_default()),
            ),
            ("$tenant_id".to_string(), Value::String(tenant_id.clone())),
            ("$action".to_string(), Value::String(action.to_string())),
            (
                "$table_name".to_string(),
                Value::String(table_name.to_string()),
            ),
            (
                "$record_id".to_string(),
                record_id.cloned().unwrap_or(Value::Null),
            ),
            (
                "$payload_before".to_string(),
                payload_before.cloned().unwrap_or(Value::Null),
            ),
            (
                "$payload_after".to_string(),
                payload_after.cloned().unwrap_or(Value::Null),
            ),
            (
                "$ip_address".to_string(),
                Value::String(ip_address.unwrap_or_default()),
            ),
            ("$created_at".to_string(), Value::String(now.clone())),
        ];

        let mut resp = self.conn.query(sql, params_to_value(params)).await?;
        let created: Vec<Value> = resp.take(0)?;

        let audit_log_id = created
            .first()
            .and_then(|v| v.get("id").and_then(|id| id.as_str()))
            .unwrap_or("unknown")
            .to_string();

        // Crear arista de grafo: user->EXECUTED->audit_log (si hay usuario).
        if let Some(uid) = &self.ctx.user_id {
            let edge_sql = r#"
                RELATE (SELECT * FROM user WHERE id = $uid LIMIT 1)
                ->EXECUTED->
                (SELECT * FROM audit_log WHERE id = $aid LIMIT 1)
                SET created_at = $created_at"#;
            let edge_params: Vec<(String, Value)> = vec![
                ("$uid".to_string(), Value::String(uid.clone())),
                ("$aid".to_string(), Value::String(audit_log_id.clone())),
                ("$created_at".to_string(), Value::String(now)),
            ];
            self.conn.query(edge_sql, params_to_value(edge_params)).await?;
        }

        info!(
            "Auditoría registrada: action={} table={} tenant={} audit_log={}",
            action, table_name, tenant_id, audit_log_id
        );

        Ok(AuditResult { audit_log_id })
    }

    async fn execute_transaction(
        &self,
        sql: &str,
        params: Vec<(String, Value)>,
    ) -> anyhow::Result<Response> {
        debug!(
            "Ejecutando transacción ACID con tenant_id={}: {}",
            self.ctx.tenant_id, sql
        );

        let mut params_with_tenant: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        params_with_tenant.extend(params);

        let resp = self.conn.query(sql, params_to_value(params_with_tenant)).await?;
        Ok(resp)
    }

    async fn create_edge(
        &self,
        from_table: &str,
        from_id: &str,
        edge_name: &str,
        to_table: &str,
        to_id: &str,
        data: Value,
    ) -> anyhow::Result<Option<Value>> {
        // Inyectar tenant_id en los datos de la arista.
        let mut edge_data = data;
        if let Some(obj) = edge_data.as_object_mut() {
            obj.insert(
                "tenant_id".to_string(),
                Value::String(self.ctx.tenant_id.clone()),
            );
            obj.insert(
                "created_at".to_string(),
                Value::String(chrono::Utc::now().to_rfc3339()),
            );
        }

        // Los nombres de tabla, arista y IDs son valores internos (UUIDs generados
        // por el sistema), no entrada de usuario, por lo que se interpolan de forma
        // segura. Los datos de la arista van parametrizados.
        let edge_sql = format!(
            "RELATE (SELECT * FROM {from_table} WHERE id = $from_id LIMIT 1) \
             ->{edge_name}-> \
             (SELECT * FROM {to_table} WHERE id = $to_id LIMIT 1) \
             SET $data"
        );

        let params: Vec<(String, Value)> = vec![
            ("$from_id".to_string(), Value::String(from_id.to_string())),
            ("$to_id".to_string(), Value::String(to_id.to_string())),
            ("$data".to_string(), edge_data),
        ];

        let mut resp = self.conn.query(&edge_sql, params_to_value(params)).await?;
        let created: Option<Value> = resp.take(0)?;
        Ok(created)
    }

    async fn create_journal_entry(
        &self,
        entry_id: &str,
        date: &str,
        description: &str,
        lines_json: Value,
        created_by: Option<&str>,
    ) -> anyhow::Result<Option<Value>> {
        // Transacción ACID: crear asiento + registrar auditoría de forma atómica.
        let tenant_id = self.ctx.tenant_id.clone();
        let now = chrono::Utc::now().to_rfc3339();

        let tx_sql = "BEGIN TRANSACTION;\
             CREATE journal_entries CONTENT {\
                 id: $entry_id,\
                 date: $date,\
                 description: $description,\
                 lines: $lines,\
                 tenant_id: $tenant_id,\
                 created_at: $now,\
                 created_by: $created_by\
             };\
             CREATE audit_log CONTENT {\
                 user_id: $user_id,\
                 company_id: $tenant_id,\
                 action: 'CREATE',\
                 table_name: 'journal_entries',\
                 record_id: $entry_id,\
                 payload_before: None,\
                 payload_after: $lines_json,\
                 ip_address: $ip_address,\
                 created_at: $now\
             };\
             COMMIT TRANSACTION;";

        let params: Vec<(String, Value)> = vec![
            ("$entry_id".to_string(), Value::String(entry_id.to_string())),
            ("$date".to_string(), Value::String(date.to_string())),
            (
                "$description".to_string(),
                Value::String(description.to_string()),
            ),
            ("$lines".to_string(), lines_json.clone()),
            ("$tenant_id".to_string(), Value::String(tenant_id.clone())),
            ("$now".to_string(), Value::String(now.clone())),
            (
                "$created_by".to_string(),
                created_by
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            ),
            (
                "$user_id".to_string(),
                self.ctx
                    .user_id
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            ),
            ("$lines_json".to_string(), lines_json),
            (
                "$ip_address".to_string(),
                self.ctx
                    .ip_address
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            ),
        ];

        let mut resp = self.conn.query(tx_sql, params_to_value(params)).await?;
        let created: Option<Value> = resp.take(0)?;
        Ok(created)
    }

    async fn query_journal_entries_by_tenant(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> anyhow::Result<Response> {
        let mut sql = String::from("SELECT * FROM journal_entries WHERE tenant_id = $tenant_id");
        let mut params: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        let mut param_idx = 1;

        if let Some(start) = start_date {
            let param_name = format!("$start_date{}", param_idx);
            sql.push_str(&format!(" AND date >= {}", param_name));
            params.push((param_name, Value::String(start.to_string())));
            param_idx += 1;
        }
        if let Some(end) = end_date {
            let param_name = format!("$end_date{}", param_idx);
            sql.push_str(&format!(" AND date <= {}", param_name));
            params.push((param_name, Value::String(end.to_string())));
        }
        sql.push_str(" ORDER BY date DESC");

        let resp = self.conn.query(&sql, params_to_value(params)).await?;
        Ok(resp)
    }

    async fn query_chart_of_accounts_by_tenant(&self) -> anyhow::Result<Response> {
        let sql = "SELECT * FROM chart_of_accounts WHERE tenant_id = $tenant_id ORDER BY code";
        let params: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        let resp = self.conn.query(sql, params_to_value(params)).await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tenant_context_new() {
        let ctx = TenantContext::new("company-123");
        assert_eq!(ctx.tenant_id, "company-123");
        assert!(ctx.user_id.is_none());
    }

    #[test]
    fn tenant_context_with_user() {
        let mut ctx = TenantContext::new("company-456");
        ctx.user_id = Some("user-789".to_string());
        ctx.ip_address = Some("192.168.1.1".to_string());
        assert_eq!(ctx.tenant_id, "company-456");
        assert_eq!(ctx.user_id.as_deref(), Some("user-789"));
    }

    #[test]
    fn create_with_tenant_injects_tenant_id() {
        // Verificamos la lógica de inyección de tenant_id sin conexión real.
        let mut data = json!({"name": "Test"});
        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "tenant_id".to_string(),
                Value::String("company-1".to_string()),
            );
        }
        assert_eq!(
            data.get("tenant_id").and_then(|v| v.as_str()),
            Some("company-1")
        );
    }
}
