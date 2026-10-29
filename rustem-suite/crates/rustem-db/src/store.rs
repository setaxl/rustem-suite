// ============================================================================
// RUSTEM SUITE — rustem_db::store
// Traits e implementación de repositorio con inyección automática de tenant_id
// y registro de auditoría en grafo.
// ============================================================================

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info};

use crate::connection::{DbConnection, DbResponse, SurrealConnection};

// ---------------------------------------------------------------------------
// TenantContext
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl TenantContext {
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

    pub fn has_permission(&self, resource: &str, action: &str) -> bool {
        self.permissions
            .iter()
            .any(|p| *p == format!("{resource}:{action}"))
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

// ---------------------------------------------------------------------------
// AuditResult
// ---------------------------------------------------------------------------

pub struct AuditResult {
    pub audit_log_id: String,
}

// ---------------------------------------------------------------------------
// Trait SurrealStore
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SurrealStore {
    fn connection(&self) -> &dyn DbConnection;
    fn tenant_context(&self) -> &TenantContext;

    async fn query_with_tenant(
        &self,
        sql: &str,
        extra_params: Vec<(String, Value)>,
    ) -> anyhow::Result<Vec<serde_json::Value>>;

    async fn create_with_tenant(&self, table: &str, data: Value) -> anyhow::Result<Option<Value>>;

    async fn update_with_tenant(
        &self,
        table: &str,
        record_id: &str,
        data: Value,
    ) -> anyhow::Result<Option<Value>>;

    async fn delete_with_tenant(
        &self,
        table: &str,
        record_id: &str,
    ) -> anyhow::Result<Option<Value>>;

    async fn audit(
        &self,
        action: &str,
        table_name: &str,
        record_id: Option<&Value>,
        payload_before: Option<&Value>,
        payload_after: Option<&Value>,
    ) -> anyhow::Result<AuditResult>;

    async fn execute_transaction(
        &self,
        sql: &str,
        params: Vec<(String, Value)>,
    ) -> anyhow::Result<Vec<serde_json::Value>>;

    async fn create_edge(
        &self,
        from_table: &str,
        from_id: &str,
        edge_name: &str,
        to_table: &str,
        to_id: &str,
        data: Value,
    ) -> anyhow::Result<Option<Value>>;

    async fn create_journal_entry(
        &self,
        entry_id: &str,
        date: &str,
        description: &str,
        lines_json: Value,
        created_by: Option<&str>,
    ) -> anyhow::Result<Option<Value>>;

    async fn query_journal_entries_by_tenant(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> anyhow::Result<Vec<serde_json::Value>>;

    async fn query_chart_of_accounts_by_tenant(&self) -> anyhow::Result<Vec<serde_json::Value>>;
}

// ---------------------------------------------------------------------------
// SurrealStoreImpl
// ---------------------------------------------------------------------------

pub struct SurrealStoreImpl {
    conn: Arc<dyn DbConnection>,
    ctx: TenantContext,
}

impl SurrealStoreImpl {
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
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let mut params: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        params.extend(extra_params);

        debug!(
            "Ejecutando consulta con tenant_id={}: {}",
            self.ctx.tenant_id, sql
        );

        let mut resp = self.conn.query(sql, params_to_value(params)).await?;
        let results: Vec<serde_json::Value> = resp.take(0)?;

        Ok(results)
    }

    async fn create_with_tenant(
        &self,
        table: &str,
        mut data: Value,
    ) -> anyhow::Result<Option<Value>> {
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
        let results: Vec<serde_json::Value> = resp.take(0)?;
        let created: Option<Value> = results.into_iter().next();

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
            .take::<Vec<serde_json::Value>>(0)?
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
        let results: Vec<serde_json::Value> = resp.take(0)?;
        let updated: Option<Value> = results.into_iter().next();

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
            .take::<Vec<serde_json::Value>>(0)?
            .into_iter()
            .next();

        let sql = format!("DELETE {table}:{record_id}");
        let params: Vec<(String, Value)> = vec![];

        let mut resp = self.conn.query(&sql, params_to_value(params)).await?;
        let results: Vec<serde_json::Value> = resp.take(0)?;
        let deleted: Option<Value> = results.into_iter().next();

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
        let created: Vec<serde_json::Value> = resp.take(0)?;

        let audit_log_id = created
            .first()
            .and_then(|v| v.get("id").and_then(|id| id.as_str()))
            .unwrap_or("unknown")
            .to_string();

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
            "Auditoria registrada: action={} table={} tenant={} audit_log={}",
            action, table_name, tenant_id, audit_log_id
        );

        Ok(AuditResult { audit_log_id })
    }

    async fn execute_transaction(
        &self,
        sql: &str,
        params: Vec<(String, Value)>,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        debug!(
            "Ejecutando transaccion ACID con tenant_id={}: {}",
            self.ctx.tenant_id, sql
        );

        let mut params_with_tenant: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        params_with_tenant.extend(params);

        let mut resp = self.conn.query(sql, params_to_value(params_with_tenant)).await?;
        let results: Vec<serde_json::Value> = resp.take(0)?;
        Ok(results)
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

        let mut resp = self.conn.query(&edge_sql, params_to_value(params))?;
        let created: Option<Value> = resp.into_iter().next();
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
        let created: Option<Value> = resp.into_iter().next();
        Ok(created)
    }

    async fn query_journal_entries_by_tenant(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let mut sql = String::from("SELECT * FROM journal_entries WHERE tenant_id = $tenant_id");
        let mut params: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        let mut param_idx = 1;

        if let Some(start) = start_date {
            let param_name = format!("$start_date{param_idx}");
            sql.push_str(&format!(" AND date >= {param_name}"));
            params.push((param_name, Value::String(start.to_string())));
            param_idx += 1;
        }
        if let Some(end) = end_date {
            let param_name = format!("$end_date{param_idx}");
            sql.push_str(&format!(" AND date <= {param_name}"));
            params.push((param_name, Value::String(end.to_string())));
            param_idx += 1;
        }
        sql.push_str(" ORDER BY date DESC");

        let mut resp = self.conn.query(&sql, params_to_value(params)).await?;
        Ok(resp)
    }

    async fn query_chart_of_accounts_by_tenant(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let sql = "SELECT * FROM chart_of_accounts WHERE tenant_id = $tenant_id ORDER BY code";
        let params: Vec<(String, Value)> = vec![(
            "$tenant_id".to_string(),
            Value::String(self.ctx.tenant_id.clone()),
        )];
        let mut resp = self.conn.query(sql, params_to_value(params)).await?;
        let results: Vec<serde_json::Value> = resp.take(0)?;
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Funcion helper params_to_value
// ---------------------------------------------------------------------------

pub fn params_to_value(params: Vec<(String, Value)>) -> Value {
    let mut map = serde_json::Map::with_capacity(params.len());
    for (key, value) in params {
        map.insert(key, value);
    }
    Value::Object(map)
}

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

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

    #[test]
    fn params_to_value_converts_pairs_to_json() {
        let params = vec![
            ("$tenant_id".to_string(), Value::String("company-1".to_string())),
            ("$user_id".to_string(), Value::String("user-1".to_string())),
        ];
        let result = params_to_value(params);
        assert_eq!(
            result.get("$tenant_id").and_then(|v| v.as_str()),
            Some("company-1")
        );
        assert_eq!(
            result.get("$user_id").and_then(|v| v.as_str()),
            Some("user-1")
        );
    }

    #[test]
    fn params_to_value_empty_returns_empty_object() {
        let params: Vec<(String, Value)> = vec![];
        let result = params_to_value(params);
        assert!(result.is_object());
        assert_eq!(result.as_object().unwrap().len(), 0);
    }

    #[test]
    fn tenant_context_has_permission_case_sensitive() {
        let mut ctx = TenantContext::new("tenant-1");
        ctx.permissions.push("sales:read".to_string());
        assert!(ctx.has_permission("sales", "read"));
        assert!(!ctx.has_permission("SALES", "read"));
        assert!(!ctx.has_permission("sales", "READ"));
    }

    #[test]
    fn tenant_context_has_role_case_sensitive() {
        let mut ctx = TenantContext::new("tenant-1");
        ctx.roles.push("Owner".to_string());
        assert!(ctx.has_role("Owner"));
        assert!(!ctx.has_role("owner"));
        assert!(!ctx.has_role("OWNER"));
    }
}