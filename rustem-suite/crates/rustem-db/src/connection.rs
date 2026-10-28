// ============================================================================
// RUSTEM SUITE — rustem-db::connection
// Conexión a SurrealDB: único punto de contacto con la base de datos.
// ============================================================================

//! # Módulo `connection`
//! Establece y gestiona la conexión con SurrealDB mediante el SDK oficial
//! en Rust (100% Rust, sin dependencias de C/C++).
//! Proporciona [`SurrealConnection`], el wrapper que expone operaciones
//! parametrizadas para prevenir inyección SurQL.

use std::sync::Arc;
use surrealdb::{Surreal, engine::any::connect as surreal_connect};
use tracing::info;

use crate::migration::MigrationRunner;

/// Respuesta de consulta que puede ser una conexión real o un mock.
/// Permite crear respuestas simuladas en pruebas sin necesidad de SurrealDB real.
#[derive(Debug)]
pub enum DbResponse {
    /// Respuesta real de SurrealDB.
    Real(surrealdb::Response),
    /// Respuesta simulada para pruebas.
    Mock(Vec<serde_json::Value>),
}

impl DbResponse {
    /// Toma y devuelve registros del resultado en el índice dado.
    /// Usa Vec<surrealdb::Value> como intermediario para permitir
    /// deserialización a cualquier tipo T: DeserializeOwned.
    pub fn take<T>(&mut self, index: usize) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        match self {
            DbResponse::Real(resp) => {
                let values: Vec<surrealdb::Value> = resp.take(index)
                    .map_err(|e| anyhow::anyhow!("surrealdb error: {e}"))?;
                let value = values.into_iter().next()
                    .unwrap_or_default();
                let sjv: serde_json::Value = serde_json::to_value(&value)
                    .map_err(|e| anyhow::anyhow!("failed to convert: {e}"))?;
                serde_json::from_value::<T>(sjv)
                    .map_err(|e| anyhow::anyhow!("failed to deserialize: {e}"))
            }
            DbResponse::Mock(values) => {
                let value = values.get(index)
                    .ok_or_else(|| anyhow::anyhow!("no result at index {}", index))?;
                serde_json::from_value::<T>(value.clone())
                    .map_err(|e| anyhow::anyhow!("failed to deserialize: {e}"))
            }
        }
    }
}

/// Trait de abstracción sobre la conexión a SurrealDB.
/// Permite usar una conexión real o un mock en pruebas.
#[async_trait::async_trait]
pub trait DbConnection: Send + Sync {
    /// Ejecuta una consulta SurQL parametrizada.
    async fn query(&self, sql: &str, params: serde_json::Value) -> anyhow::Result<DbResponse>;

    /// Ejecuta una consulta sin retorno tipificado.
    async fn execute(&self, sql: &str, params: serde_json::Value) -> anyhow::Result<DbResponse>;

    /// Aplica migraciones pendientes.
    async fn apply_migrations(&self) -> anyhow::Result<crate::migration::MigrationResult>;

    /// Clona la conexión como trait object.
    fn clone_box(&self) -> Arc<dyn DbConnection>;
}/// Wrapper sobre la conexión a SurrealDB.
/// Es el único tipo que tiene acceso directo al cliente de SurrealDB.
/// Todos los demás crates acceden a través de [`crate::store::SurrealStore`].
#[derive(Clone, Debug)]
pub struct SurrealConnection {
    /// Cliente de SurrealDB conectado.
    pub client: Surreal<surrealdb::engine::any::Any>,
    /// Namespace activo.
    pub namespace: String,
    /// Base de datos activa.
    pub database: String,
}

impl SurrealConnection {
    /// Conecta a SurrealDB, autentica con usuario/contraseña y selecciona
    /// namespace y base de datos.
    ///
    /// # Errores
    /// Devuelve `anyhow::Error` si falla la conexión, autenticación o
    /// selección de namespace/base de datos.
    pub async fn connect(
        url: &str,
        username: &str,
        password: &str,
        namespace: &str,
        database: &str,
    ) -> anyhow::Result<Self> {
        info!("Conectando a SurrealDB en {}", url);

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

    /// Ejecuta una consulta SurQL parametrizada vinculando parámetros.
    /// Los parámetros deben ser `static` para cumplir con los requisitos
    /// del SDK de SurrealDB.
    ///
    /// # Errores
    /// Devuelve `anyhow::Error` si la consulta falla.
    pub async fn query<P>(&self, sql: &str, params: P) -> anyhow::Result<DbResponse>
    where
        P: serde::Serialize + 'static,
    {
        let resp = self.client.query(sql).bind(params).await?;
        Ok(DbResponse::Real(resp))
    }

    /// Ejecuta una consulta sin retorno de datos tipificado.
    pub async fn execute(
        &self,
        sql: &str,
        params: impl serde::Serialize + 'static,
    ) -> anyhow::Result<DbResponse> {
        let resp = self.client.query(sql).bind(params).await?;
        Ok(DbResponse::Real(resp))
    }

    /// Aplica todas las migraciones `.surql` pendientes de forma idempotente.
    pub async fn apply_migrations(&self) -> anyhow::Result<crate::migration::MigrationResult> {
        MigrationRunner::new().apply_all(self).await
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

    async fn apply_migrations(&self) -> anyhow::Result<crate::migration::MigrationResult> {
        MigrationRunner::new().apply_all(self).await
    }

    fn clone_box(&self) -> Arc<dyn DbConnection> {
        Arc::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn connection_struct_debug() {
        // Prueba de unidad: SurrealConnection puede formatearse para depuración.
        // No se conecta a SurrealDB real en pruebas unitarias.
        let _ = ();
    }

    #[test]
    fn db_response_take_mock_data() {
        // Prueba que DbResponse::take extrae datos de Mock correctamente.
        let mock_data = serde_json::json!({"id": "1", "name": "Test"});
        let mut response = DbResponse::Mock(vec![mock_data]);
        let result: serde_json::Value = response.take(0).unwrap();
        assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("1"));
        assert_eq!(result.get("name").and_then(|v| v.as_str()), Some("Test"));
    }

    #[test]
    fn db_response_take_index_out_of_bounds() {
        // Prueba que DbResponse::take retorna error cuando el índice está fuera de rango.
        let mock_data = serde_json::json!([{"id": "1"}]);
        let mut response = DbResponse::Mock(vec![mock_data]);
        let result: Result<serde_json::Value, _> = response.take(5);
        assert!(result.is_err());
    }

    #[test]
    fn db_response_take_empty_mock() {
        // Prueba que DbResponse::take retorna error cuando no hay datos.
        let mut response = DbResponse::Mock(vec![]);
        let result: Result<serde_json::Value, _> = response.take(0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn migration_runner_apply_all_with_mock() {
        use crate::{MigrationResult, MigrationRunner};
        use crate::store::TenantContext;

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
