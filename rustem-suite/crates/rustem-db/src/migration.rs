// ============================================================================
// RUSTEM SUITE — rustem_db::migration
// Gestor de migraciones .surql versionadas e idempotentes.
// ============================================================================

use async_trait::async_trait;
use rust_embed::RustEmbed;
use tracing::{info, warn};

use crate::connection::DbConnection;
use crate::DbResponse;

// ---------------------------------------------------------------------------
// MigrationResult y MigrationRunner
// ---------------------------------------------------------------------------

pub struct MigrationResult {
    pub applied: usize,
    pub skipped: usize,
}

pub struct MigrationRunner;

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self
    }

    pub async fn apply_all(
        &self,
        conn: &dyn DbConnection,
    ) -> anyhow::Result<MigrationResult> {
        let mut applied = 0usize;
        let skipped = 0usize;

        let mut migration_files: Vec<String> = Migrations::iter().map(|p| p.to_string()).collect();
        migration_files.sort();

        for path in migration_files {
            if !path.ends_with(".surql") {
                continue;
            }

            info!("Aplicando migracion: {path}");

            let content = match Migrations::get(&path) {
                Some(c) => String::from_utf8_lossy(&c.data).to_string(),
                None => {
                    warn!("No se pudo leer la migracion embedida: {path}");
                    continue;
                }
            };

            let _resp = conn.execute(content.as_str(), serde_json::json!([])).await?;

            applied += 1;
            info!("Migracion aplicada: {path}");
        }

        Ok(MigrationResult { applied, skipped })
    }
}

#[derive(RustEmbed)]
#[folder = "../../migrations"]
struct Migrations;

// ---------------------------------------------------------------------------
// Pruebas unitarias
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_runner_creates() {
        let runner = MigrationRunner::new();
        let _ = runner;
    }

    #[test]
    fn migration_files_are_embedded() {
        let files: Vec<String> = Migrations::iter().map(|p| p.to_string()).collect();
        assert!(
            !files.is_empty(),
            "Se esperaba al menos una migracion embedida"
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