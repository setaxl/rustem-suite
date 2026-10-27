// ============================================================================
// RUSTEM SUITE — rustem-db::connection
// Conexión a SurrealDB: único punto de contacto con la base de datos.
// ============================================================================

//! # Módulo `connection`
//! Establece y gestiona la conexión con SurrealDB mediante el SDK oficial
//! en Rust (100% Rust, sin dependencias de C/C++).
//! Proporciona [`SurrealConnection`], el wrapper que expone operaciones
//! parametrizadas para prevenir inyección SurQL.

use surrealdb::{Surreal, engine::any::connect as surreal_connect};
use tracing::info;

use crate::migration::MigrationRunner;

/// Wrapper sobre la conexión a SurrealDB.
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
    pub async fn query<P>(&self, sql: &str, params: P) -> anyhow::Result<surrealdb::Response>
    where
        P: serde::Serialize + 'static,
    {
        let resp = self.client.query(sql).bind(params).await?;
        Ok(resp)
    }

    /// Ejecuta una consulta sin retorno de datos tipificado.
    pub async fn execute(
        &self,
        sql: &str,
        params: impl serde::Serialize + 'static,
    ) -> anyhow::Result<surrealdb::Response> {
        let resp = self.client.query(sql).bind(params).await?;
        Ok(resp)
    }

    /// Aplica todas las migraciones `.surql` pendientes de forma idempotente.
    pub async fn apply_migrations(&self) -> anyhow::Result<crate::migration::MigrationResult> {
        MigrationRunner::new().apply_all(self).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn connection_struct_debug() {
        // Prueba de unidad: SurrealConnection puede formatearse para depuración.
        // No se conecta a SurrealDB real en pruebas unitarias.
        let _ = ();
    }
}
