-- Tabla principal de memorias
CREATE TABLE IF NOT EXISTS memories (
    id BLOB PRIMARY KEY NOT NULL,
    project TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'project',
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    what TEXT,
    why TEXT,
    context TEXT,
    learned TEXT,
    memory_type TEXT NOT NULL,
    importance TEXT NOT NULL DEFAULT 'medium',
    tags TEXT NOT NULL DEFAULT '[]',
    topic_key TEXT,
    access_count INTEGER NOT NULL DEFAULT 0,
    revision_count INTEGER NOT NULL DEFAULT 0,
    duplicate_count INTEGER NOT NULL DEFAULT 0,
    normalized_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_accessed_at TEXT,
    last_seen_at TEXT,
    deleted_at TEXT
);

-- Índices para la tabla memories
CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project);
CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope);
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance);
CREATE INDEX IF NOT EXISTS idx_memories_topic ON memories(topic_key);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
CREATE INDEX IF NOT EXISTS idx_memories_updated ON memories(updated_at);
CREATE INDEX IF NOT EXISTS idx_memories_deleted ON memories(deleted_at);
CREATE INDEX IF NOT EXISTS idx_memories_hash ON memories(normalized_hash);
CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_project_hash ON memories(project, normalized_hash) WHERE normalized_hash IS NOT NULL;

-- Tabla de relaciones entre memorias
CREATE TABLE IF NOT EXISTS memory_relations (
    id BLOB PRIMARY KEY NOT NULL,
    sync_id TEXT NOT NULL,
    source_id BLOB NOT NULL,
    target_id BLOB NOT NULL,
    relation_type TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    judgment_status TEXT NOT NULL DEFAULT 'pending',
    reason TEXT,
    evidence TEXT,
    marked_by_actor TEXT NOT NULL DEFAULT 'system',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (source_id) REFERENCES memories(id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES memories(id) ON DELETE CASCADE,
    UNIQUE(source_id, target_id, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_relations_source ON memory_relations(source_id);
CREATE INDEX IF NOT EXISTS idx_relations_target ON memory_relations(target_id);
CREATE INDEX IF NOT EXISTS idx_relations_type ON memory_relations(relation_type);
CREATE INDEX IF NOT EXISTS idx_relations_sync ON memory_relations(sync_id);
CREATE INDEX IF NOT EXISTS idx_relations_status ON memory_relations(judgment_status);

-- Tabla de sesiones
CREATE TABLE IF NOT EXISTS sessions (
    id BLOB PRIMARY KEY NOT NULL,
    project TEXT NOT NULL,
    directory TEXT,
    summary TEXT,
    memory_ids TEXT NOT NULL DEFAULT '[]',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);

-- Tabla de prompts de usuario
CREATE TABLE IF NOT EXISTS user_prompts (
    id BLOB PRIMARY KEY NOT NULL,
    session_id BLOB,
    content TEXT NOT NULL,
    project TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_prompts_project ON user_prompts(project);
CREATE INDEX IF NOT EXISTS idx_prompts_session ON user_prompts(session_id);
CREATE INDEX IF NOT EXISTS idx_prompts_created ON user_prompts(created_at);

-- Tabla para sincronización con git (sync_chunks)
CREATE TABLE IF NOT EXISTS sync_chunks (
    id BLOB PRIMARY KEY NOT NULL,
    project TEXT NOT NULL,
    hash TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_sync_project ON sync_chunks(project);
CREATE INDEX IF NOT EXISTS idx_sync_hash ON sync_chunks(hash);

-- PRAGMAs de rendimiento se aplican en Database::open, no aquí
