-- Conflict candidate detection system
-- Inspired by Engram's mem_judge and mem_compare

-- Potential conflict candidates detected on save for LLM judgment
CREATE TABLE IF NOT EXISTS relation_candidates (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id       TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    target_id       TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    reason          TEXT NOT NULL,              -- why they might conflict (topic_key, content, title similarity, etc.)
    match_score     REAL NOT NULL DEFAULT 0.0,  -- 0.0 - 1.0 similarity score
    candidate_type  TEXT NOT NULL DEFAULT 'auto', -- 'auto', 'topic_key', 'semantic', 'title'
    judgment_status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'judged', 'dismissed'
    judged_relation TEXT,                        -- 'conflicts_with', 'supersedes', 'extends', 'compatible', etc.
    judged_reason   TEXT,                        -- LLM's reasoning for the judgment
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    judged_at       TEXT,
    UNIQUE(source_id, target_id, candidate_type)
);

CREATE INDEX IF NOT EXISTS idx_candidates_source ON relation_candidates(source_id);
CREATE INDEX IF NOT EXISTS idx_candidates_target ON relation_candidates(target_id);
CREATE INDEX IF NOT EXISTS idx_candidates_status ON relation_candidates(judgment_status);
CREATE INDEX IF NOT EXISTS idx_candidates_source_target ON relation_candidates(source_id, target_id);

-- LLM judgment provenance
CREATE TABLE IF NOT EXISTS conflict_judgments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    candidate_id    INTEGER NOT NULL REFERENCES relation_candidates(id) ON DELETE CASCADE,
    memory_id_a     TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    memory_id_b     TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    proposed_relation TEXT NOT NULL,
    confidence      REAL NOT NULL DEFAULT 0.0,
    reasoning       TEXT,
    evidence        TEXT,                        -- JSON array of evidence snippets
    judged_by       TEXT NOT NULL DEFAULT 'llm', -- 'llm', 'agent', 'user'
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_judgments_candidate ON conflict_judgments(candidate_id);
CREATE INDEX IF NOT EXISTS idx_judgments_a ON conflict_judgments(memory_id_a);
CREATE INDEX IF NOT EXISTS idx_judgments_b ON conflict_judgments(memory_id_b);
