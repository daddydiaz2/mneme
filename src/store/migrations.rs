use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

/// Ejecuta las migraciones de base de datos hasta la última versión.
pub fn run_migrations(conn: &mut Connection) -> crate::error::Result<()> {
    let migrations = Migrations::new(vec![
        M::up(include_str!("../../migrations/001_initial.sql")),
        M::up(include_str!("../../migrations/002_fts5.sql")),
        M::up(include_str!("../../migrations/003_vectors.sql")),
        M::up(include_str!("../../migrations/004_tools.sql")),
        M::up(include_str!("../../migrations/005_sync.sql")),
        M::up(include_str!("../../migrations/006_sync_origin.sql")),
        M::up(include_str!("../../migrations/007_encryption.sql")),
        M::up(include_str!(
            "../../migrations/008_fts5_encryption_aware.sql"
        )),
        M::up(include_str!("../../migrations/009_entities.sql")),
        M::up(include_str!("../../migrations/010_conflict_candidates.sql")),
        M::up(include_str!("../../migrations/011_temporal_validity.sql")),
        M::up(include_str!("../../migrations/012_failure_mining.sql")),
    ]);
    migrations.to_latest(conn)?;
    Ok(())
}
