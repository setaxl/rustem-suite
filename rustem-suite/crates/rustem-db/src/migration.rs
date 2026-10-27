// ============================================================================
// RUSTEM SUITE — rustem-db::migration
// Gestor de migraciones .surql versionadas e idempotentes.
// ============================================================================

//! # Módulo `migration`
//! Sistema de migraciones de esquema para SurrealDB.
//! Las migraciones son archivos `.surql` versionadas (ej. `001_init.surql`)
//! embebidas en el binario con `rust-embed` y aplicadas de forma idempotente
//! (cada sentencia usa `IF NOT EXISTS`).

use rust_embed::RustEmbed;
use tracing::{info, warn};

/// Archivos de migración embebidos en el binario.
/// El directorio `../../migrations` está relativo a la raíz del crate.
#[derive(RustEmbed)]
#[folder = "../../migrations"]
struct Migrations;

/// Resultado de la aplicación de migraciones.
pub struct MigrationResult {
    /// Número de migraciones aplicadas.
    pub applied: usize,
    /// Número de migraciones omitidas (ya aplicadas).
    pub skipped: usize,
}

/// Ejecutor de migraciones idempotentes.
pub struct MigrationRunner;

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRunner {
    /// Crea un nuevo ejecutor de migraciones.
    pub fn new() -> Self {
        Self
    }

    /// Aplica todas las migraciones embebidas en orden alfabético.
    /// Cada archivo `.surql` se ejecuta como un bloque; las sentencias
    /// individuales deben ser idempotentes (`DEFINE ... IF NOT EXISTS`).
    ///
    /// # Errores
    /// Devuelve `anyhow::Error` si alguna migración falla.
    pub async fn apply_all(
        &self,
        conn: &crate::connection::SurrealConnection,
    ) -> anyhow::Result<MigrationResult> {
        let mut applied = 0usize;
        let skipped = 0usize;

        // rust-embed devuelve las rutas de los archivos embebidos.
        let mut migration_files: Vec<String> = Migrations::iter().map(|p| p.to_string()).collect();
        migration_files.sort();

        for path in migration_files {
            if !path.ends_with(".surql") {
                continue;
            }

            info!("Aplicando migración: {}", path);

            let content = match Migrations::get(&path) {
                Some(c) => String::from_utf8_lossy(&c.data).to_string(),
                None => {
                    warn!("No se pudo leer la migración embebida: {}", path);
                    continue;
                }
            };

            // Ejecutar el contenido completo del archivo .surql.
            // SurrealDB ejecuta múltiples sentencias separadas por punto y coma.
            let _resp = conn.client.query(content.as_str()).await?;

            applied += 1;
            info!("Migración aplicada: {}", path);
        }

        Ok(MigrationResult { applied, skipped })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_runner_creates() {
        // Prueba de unidad: MigrationRunner se puede instanciar.
        let runner = MigrationRunner::new();
        let _ = runner;
        let _ = ();
    }

    #[test]
    fn migration_files_are_embedded() {
        // Verifica que al menos un archivo de migración esté embebido.
        let files: Vec<String> = Migrations::iter().map(|p| p.to_string()).collect();
        assert!(
            !files.is_empty(),
            "Se esperaba al menos una migración embebida"
        );
        let surql_files: Vec<&str> = files
            .iter()
            .filter(|f| f.ends_with(".surql"))
            .map(|s| s.as_str())
            .collect();
        assert!(
            !surql_files.is_empty(),
            "Se esperaba al menos un archivo .surql"
        );
    }
}
