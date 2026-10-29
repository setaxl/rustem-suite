// ============================================================================
// RUSTEM SUITE — rustem_db::connection
// Conexion a SurrealDB: unico punto de contacto con la base de datos.
// ============================================================================

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use surrealdb::{Surreal, engine::any::connect as surreal_connect};
use tracing::info;

use crate::migration::{MigrationResult, MigrationRunner};

// ---------------------------------------------------------------------------
// DbResponse
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DbResponse {
    Real(surrealdb::Response),
    Mock(Vec<serde_json::Value>),
}

impl DbResponse {
    pub fn take<T>(&mut self, index: usize) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        match self {
            DbResponse::Real(resp) => {
                let sv: surrealdb::Value = resp.take(index)
                    .map_err(|e| anyhow::anyhow!("error de surrealdb: {e}"))?;
                serde_json::to_value(&sv)
                    .map_err(|e| anyhow::anyhow!("fallo al convertir: {e}"))
                    .and_then(|sv| serde_json::from_value(sv)
                        .map_err(|e| anyhow::anyhow!("fallo al deserializar: {e}")))
            }
            DbResponse::Mock(values) => {
                let value = values.get(index)
                    .ok_or_else(|| anyhow::anyhow!("no hay resultado en el indice {index}"))?;
                serde_json::from_value(value.clone())
                    .map_err(|e| anyhow::anyhow!("fallo al deserializar: {e}"))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trait DbConnection
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait DbConnection: Send + Sync {
    async fn query(&self, sql: &str, params: serde_json::Value) -> anyhow::Result<DbResponse>;
    async fn execute(&self, sql: &str, params: serde_json::Value) -> anyhow::Result<DbResponse>;
    async fn apply_migrations(&self) -> anyhow::Result<MigrationResult>;
    fn clone_box(&self) -> Arc<dyn DbConnection>;
}

// ---------------------------------------------------------------------------
// SurrealConnection
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SurrealConnection {
    pub client: Surreal<surrealdb::engine::any::Any>,
    pub namespace: String,
    pub database: String,
}

impl SurrealConnection {
    pub async fn connect(
        url: &str,
        username: &str,
        password: &str,
        namespace: &str,
        database: &str,
    ) -> anyhow::Result<Self> {
        info!("Conectando a SurrealDB en {url}");

        let client = surreal_connect(url).await?;
        client
            .signin(surrealdb::opt::auth::Root { username, password })
            .await?;
        client.use_ns(namespace).use_db(database).await?;

        info!(
            "Conectado a SurrealDB (namespace={}, database={})",
            namespace, database
        );

        Ok(Self {
            client,
            namespace: namespace.to_string(),
            database: database.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl DbConnection for SurrealConnection {
    async fn query(&self, sql: &str, params: serde_json::Value) -> anyhow::Result<DbResponse> {
        let resp = self.client.query(sql).bind(params).await?;
        Ok(DbResponse::Real(resp))
    }

    async fn execute(&self, sql: &str, params: serde_json::Value) -> anyhow::Result<DbResponse> {
        let resp = self.client.query(sql).bind(params).await?;
        Ok(DbResponse::Real(resp))
    }

    async fn apply_migrations(&self) -> anyhow::Result<MigrationResult> {
        MigrationRunner::new().apply_all(self).await
    }

    fn clone_box(&self) -> Arc<dyn DbConnection> {
        Arc::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_struct_debug() {
        let _ = ();
    }

    #[test]
    fn db_response_take_mock_data() {
        let mock_data = serde_json::json!({"id": "1", "name": "Test"});
        let mut response = DbResponse::Mock(vec![mock_data]);
        let result: serde_json::Value = response.take(0).unwrap();
        assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("1"));
        assert_eq!(result.get("name").and_then(|v| v.as_str()), Some("Test"));
    }

    #[test]
    fn db_response_take_index_out_of_bounds() {
        let mock_data = serde_json::json!([{"id": "1"}]);
        let mut response = DbResponse::Mock(vec![mock_data]);
        let result: Result<serde_json::Value, _> = response.take(5);
        assert!(result.is_err());
    }

    #[test]
    fn db_response_take_empty_mock() {
        let mut response = DbResponse::Mock(vec![]);
        let result: Result<serde_json::Value, _> = response.take(0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn migration_runner_apply_all_with_mock() {
        struct MockMigrationConnection;

        #[async_trait::async_trait]
        impl DbConnection for MockMigrationConnection {
            async fn query(&self, _sql: &str, _params: serde_json::Value) -> anyhow::Result<DbResponse> {
                Ok(DbResponse::Mock(vec![]))
            }

            async fn execute(&self, _sql: &str, _params: serde_json::Value) -> anyhow::Result<DbResponse> {
                Ok(DbResponse::Mock(vec![]))
            }

            async fn apply_migrations(&self) -> anyhow::Result<MigrationResult> {
                Ok(MigrationResult { applied: 0, skipped: 0 })
            }

            fn clone_box(&self) -> Arc<dyn DbConnection> {
                Arc::new(Self)
            }
        }

        let mock = Arc::new(MockMigrationConnection);
        let result = mock.apply_migrations().await;
        assert!(result.is_ok());
        let migration_result = result.unwrap();
        assert_eq!(migration_result.applied, 0);
        assert_eq!(migration_result.skipped, 0);
    }
}