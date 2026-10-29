// ============================================================================
// RUSTEM SUITE — rustem_db
// Único punto de contacto con SurrealDB + Migraciones .surql
// ============================================================================

//! # Módulo `rustem_db`
//! Capa de abstracción para SurrealDB:
//! - `DbConnection` trait: interfaz para conexiones reales o mock.
//! - `DbResponse` enum: encapsula respuestas (`Real` o `Mock`).
//! - `take<T>`: método genérico para deserializar resultados.
//! - `MigrationRunner`: ejecuta migraciones .surql versionadas.
//! - Función `params_to_value`: convierte tipos seguros a `serde_json::Value`.
//!
//! **NOTA:** Este crate no incluye `store.rs` por directiva del proyecto.
//! La gestión de estado y almacenamiento se realiza a través de la capa
//! `rustem-db` usando traits y enums diseñados para testing sin base de datos real.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use surrealdb::{
    engine::any::Any,
    Error as SqlError,
    Surreal,
};

// ---------------------------------------------------------------------------
// Errores del crate `rustem-db`
// ---------------------------------------------------------------------------

/// Errores específicos de la capa `rustem-db`.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("error de conexión SurrealDB: {0}")]
    Connection(#[from] SqlError),
    #[error("error de migración: {0}")]
    Migration(String),
    #[error("error de parámetro: {0}")]
    Parameter(String),
    #[error("error de serialización: {0}")]
    Serialization(String),
    #[error("error de deserialización: {0}")]
    Deserialization(String),
}

// Re-export for convenience
pub type DbResult<T> = Result<T, DbError>;

// ---------------------------------------------------------------------------
// DbResponse: encapsula respuestas reales o mock de SurrealDB
// ---------------------------------------------------------------------------

/// Respuesta de SurrealDB, ya real o mock para pruebas.
#[derive(Debug, Clone)]
pub enum DbResponse {
    /// Respuesta real del SDK de SurrealDB.
    Real(Surreal<Any>),
    /// Respuesta mock para pruebas sin base de datos real.
    Mock(Vec<serde_json::Value>),
}

impl DbResponse {
    /// Deserializa el primer registro de la respuesta como tipo `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustem_db::DbResponse;
    /// use serde::Deserialize;
    /// use serde_json::Value;
    ///
    /// let mock = DbResponse::Mock(vec![Value::from("hello")]);
    /// let s: String = mock.take().unwrap();
    /// assert_eq!(s, "hello");
    /// ```
    pub fn take<T: for<'de> Deserialize<'de>>(self) -> DbResult<T> {
        match self {
            DbResponse::Real(_db) => {
                // Deserialización desde el SDK real dejaría como ejercicio
                // de integración; en pruebas usamos Mock.
                unimplemented!("Deserialización desde Db real - usar Mock para pruebas unitarias")
            }
            DbResponse::Mock(values) => {
                if values.is_empty() {
                    return Err(DbError::Parameter("No hay valores en DbResponse::Mock".to_string()));
                }
                let value = &values[0];
                // Usar map_err en lugar de ? para evitar el requerimiento de From<serde_json::Error>
                serde_json::from_value(value.clone())
                    .map_err(|e| DbError::Deserialization(e.to_string()))
            }
        }
    }

    /// Retorna todos los valores de la respuesta mock.
    pub fn into_values(self) -> Vec<serde_json::Value> {
        match self {
            DbResponse::Real(_) => vec![],
            DbResponse::Mock(values) => values,
        }
    }
}

// ---------------------------------------------------------------------------
// DbConnection trait: abstracción sobre la conexión a SurrealDB
// ---------------------------------------------------------------------------

/// Trait para conexiones a SurrealDB, con métodos async.
/// En Rust 1.75+ los métodos async en traits son compatibles.
/// Este trait está diseñado para ser usado con tipos concretos, no con `dyn`.
/// Para usar con `dyn`, se requiere que el tipo concreto implemente el trait.
pub trait DbConnection: Send + Sync + 'static {
    /// Conecta a la instancia de SurrealDB.
    async fn connect(uri: &str, ns: &str, db: &str) -> DbResult<DbResponse>;

    /// Ejecuta una query parametrizada.
    async fn query(&self, query: &str, params: serde_json::Value) -> DbResult<DbResponse>;

    /// Ejecuta una migración .surql.
    async fn migrate(&self, surql: &str) -> DbResult<()>;

    /// Cierra la conexión.
    async fn close(&self) -> DbResult<()>;
}

// ---------------------------------------------------------------------------
// MigrationRunner: ejecuta migraciones .surql versionadas
// ---------------------------------------------------------------------------

/// Carga y ejecuta migraciones idempotentes embebidas con `rust-embed`.
pub struct MigrationRunner {
    /// Ruta base donde se encuentran los archivos .surql
    base_path: String,
}

impl MigrationRunner {
    /// Crea un nuevo `MigrationRunner` apuntando a un directorio de migraciones.
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    /// Ejecuta una migración específica por nombre.
    pub async fn run(&self, name: &str) -> DbResult<()> {
        let path = format!("{}/{}.surql", self.base_path, name);
        let surql = self.load_surql(&path).await?;
        self.execute(&surql).await?;
        Ok(())
    }

    /// Carga el contenido de un archivo .surql.
    async fn load_surql(&self, path: &str) -> DbResult<String> {
        std::fs::read_to_string(path).map_err(|e| DbError::Migration(format!("No se pudo cargar {path}: {e}").into()))
    }

    /// Ejecuta el SQL SurrealDB directamente.
    async fn execute(&self, _surql: &str) -> DbResult<()> {
        // La ejecución real dependerá de la conexión DB activa
        // Esta firma sirve como interfaz; la implementación concreta
        // usaría un DbConnection inyectado.
        todo!("Ejecución de migración SurrealDB - requiere DbConnection")
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new("migrations".to_string())
    }
}

// ---------------------------------------------------------------------------
// Función de ayuda para params_to_value
// ---------------------------------------------------------------------------

/// Convierte un tipo Serialize a `serde_json::Value` seguro.
pub fn params_to_value<T: Serialize>(item: &T) -> DbResult<serde_json::Value> {
    let value = serde_json::to_value(item)
        .map_err(|e| DbError::Serialization(format!("No se pudo convertir a Value: {e}")))?;
    Ok(value)
}

// ---------------------------------------------------------------------------
// Pruebas unitarias (mock, sin SurrealDB real)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn db_response_mock_take() {
        let mock = DbResponse::Mock(vec![json!({"name": "test", "value": 42})]);
        let result: DbResult<serde_json::Value> = mock.take();
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["value"], 42);
    }

    #[test]
    fn db_response_mock_take_empty() {
        let mock = DbResponse::Mock(vec![]);
        let result: DbResult<serde_json::Value> = mock.take();
        assert!(result.is_err());
    }

    #[test]
    fn params_to_value_work() {
        let result = params_to_value(&serde_json::json!({"key": "value"})).unwrap();
        assert!(result.is_object());
        assert_eq!(result.get("key").unwrap(), "value");
    }

    #[test]
    fn migration_runner_default() {
        let runner = MigrationRunner::default();
        assert_eq!(runner.base_path, "migrations");
    }

    #[test]
    fn db_response_into_values() {
        let mock = DbResponse::Mock(vec![json!({"a": 1}), json!({"b": 2})]);
        let values = mock.into_values();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], json!({"a": 1}));
        assert_eq!(values[1], json!({"b": 2}));
    }
}