CREATE TABLE IF NOT EXISTS sync_state (
    memory_id    TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
    automerge_id TEXT NOT NULL UNIQUE,
    doc_bytes    BLOB NOT NULL,
    last_synced  TEXT,
    is_tombstone INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sync_automerge_id ON sync_state(automerge_id);
CREATE INDEX IF NOT EXISTS idx_sync_tombstones   ON sync_state(is_tombstone);

CREATE TABLE IF NOT EXISTS sync_peers (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    transport    TEXT NOT NULL,
    address      TEXT NOT NULL,
    project      TEXT NOT NULL,
    last_sync    TEXT,
    last_status  TEXT,
    auto_sync    INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_peers_project ON sync_peers(project);

CREATE TABLE IF NOT EXISTS sync_log (
    id           TEXT PRIMARY KEY,
    peer_id      TEXT REFERENCES sync_peers(id),
    direction    TEXT NOT NULL,
    project      TEXT NOT NULL,
    memories_sent     INTEGER NOT NULL DEFAULT 0,
    memories_received INTEGER NOT NULL DEFAULT 0,
    conflicts_resolved INTEGER NOT NULL DEFAULT 0,
    duration_ms  INTEGER,
    status       TEXT NOT NULL,
    error        TEXT,
    started_at   TEXT NOT NULL,
    finished_at  TEXT
);
