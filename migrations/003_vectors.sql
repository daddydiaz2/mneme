CREATE TABLE IF NOT EXISTS memory_embeddings (
    memory_id   TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
    embedding   BLOB NOT NULL,
    model_name  TEXT NOT NULL,
    dimensions  INTEGER NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_embeddings_model ON memory_embeddings(model_name);

CREATE TABLE IF NOT EXISTS embedding_config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO embedding_config VALUES
    ('model_name', 'BAAI/bge-small-en-v1.5'),
    ('dimensions', '384'),
    ('enabled', 'true');
