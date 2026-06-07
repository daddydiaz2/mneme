-- Entity extraction and linking system
-- Inspired by Mem0's entity linking and Graphiti's entity tracking

-- Named entities extracted from memory content
CREATE TABLE IF NOT EXISTS memory_entities (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    entity_name TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'concept',  -- concept, person, library, file_path, url, technology, framework, etc.
    confidence  REAL NOT NULL DEFAULT 1.0,         -- 0.0 - 1.0 extraction confidence
    context     TEXT,                               -- surrounding context snippet where entity was found
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_entities_memory ON memory_entities(memory_id);
CREATE INDEX IF NOT EXISTS idx_entities_name ON memory_entities(entity_name);
CREATE INDEX IF NOT EXISTS idx_entities_type ON memory_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_name_type ON memory_entities(entity_name, entity_type);

-- Cross-memory entity links for retrieval boosting
CREATE TABLE IF NOT EXISTS entity_links (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_name     TEXT NOT NULL,
    entity_type     TEXT NOT NULL DEFAULT 'concept',
    source_memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    target_memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    link_strength    REAL NOT NULL DEFAULT 1.0,     -- based on co-occurrence frequency
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_name, source_memory_id, target_memory_id)
);

CREATE INDEX IF NOT EXISTS idx_entity_links_name ON entity_links(entity_name);
CREATE INDEX IF NOT EXISTS idx_entity_links_source ON entity_links(source_memory_id);
CREATE INDEX IF NOT EXISTS idx_entity_links_target ON entity_links(target_memory_id);
CREATE INDEX IF NOT EXISTS idx_entity_links_name_source ON entity_links(entity_name, source_memory_id);
