use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::error::{MnemeError, Result};
use crate::sync::protocol::MemoryChangeset;

/// Transporte basado en archivos para sincronizacion.
pub struct FileTransport {
    directory: PathBuf,
}

/// Estado de exportacion de un proyecto.
#[derive(Debug, Clone, Default)]
pub struct ExportStats {
    /// Cantidad de memorias exportadas.
    pub memories_exported: u32,
    /// Bytes escritos al archivo.
    pub bytes_written: u64,
}

/// Estado de importacion de un archivo.
#[derive(Debug, Clone, Default)]
pub struct ImportStats {
    /// Cantidad de memorias importadas.
    pub memories_imported: u32,
    /// Cantidad de archivos procesados.
    pub files_processed: u32,
}

impl FileTransport {
    /// Crea un nuevo FileTransport.
    pub fn new(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }

    /// Exporta cambios a un archivo comprimido con zstd.
    pub fn export(
        &self,
        project: &str,
        changes: &[MemoryChangeset],
    ) -> Result<(PathBuf, ExportStats)> {
        let filename = format!("{}_{}.zst", project, chrono::Utc::now().timestamp());
        let path = self.directory.join(&filename);

        let payload = serde_json::to_vec(changes)?;
        let compressed =
            zstd::encode_all(std::io::Cursor::new(payload), 3).map_err(MnemeError::Io)?;

        fs::write(&path, &compressed)?;

        let stats = ExportStats {
            memories_exported: changes.len() as u32,
            bytes_written: compressed.len() as u64,
        };

        tracing::info!(
            "exported {} memories to {} ({} bytes)",
            stats.memories_exported,
            path.display(),
            stats.bytes_written
        );

        Ok((path, stats))
    }

    /// Importa cambios pendientes desde archivos en el directorio.
    pub fn import_pending(
        &self,
        project: &str,
    ) -> Result<(Vec<MemoryChangeset>, HashMap<String, bool>)> {
        let mut all_changes = Vec::new();
        let mut processed = HashMap::new();

        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if !name.starts_with(project)
                || path.extension().and_then(|s| s.to_str()) != Some("zst")
            {
                continue;
            }

            if processed.get(&name).copied().unwrap_or(false) {
                continue;
            }

            let compressed = fs::read(&path)?;
            let decompressed =
                zstd::decode_all(std::io::Cursor::new(compressed)).map_err(MnemeError::Io)?;
            let changes: Vec<MemoryChangeset> = serde_json::from_slice(&decompressed)?;
            all_changes.extend(changes);
            processed.insert(name, false);
        }

        Ok((all_changes, processed))
    }

    /// Marca un archivo como aplicado (renombra a .applied).
    pub fn mark_applied(&self, filename: &str) -> Result<()> {
        let src = self.directory.join(format!("{}.zst", filename));
        let dst = self.directory.join(format!("{}.applied", filename));
        if src.exists() {
            fs::rename(&src, &dst)?;
            tracing::info!("marked applied: {}", filename);
        }
        Ok(())
    }

    /// Lista archivos pendientes de importar.
    pub fn list_pending(&self, project: &str) -> Result<Vec<PathBuf>> {
        let mut pending = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if name.starts_with(project) && path.extension().and_then(|s| s.to_str()) == Some("zst")
            {
                pending.push(path);
            }
        }
        Ok(pending)
    }
}
