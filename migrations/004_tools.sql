-- Feedback scores para relevance learning
CREATE TABLE IF NOT EXISTS memory_feedback (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    is_useful   INTEGER NOT NULL,  -- 1 = useful, 0 = not useful
    reason      TEXT,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_feedback_memory ON memory_feedback(memory_id);

-- Campos para deprecación
ALTER TABLE memories ADD COLUMN deprecated_at TEXT;
ALTER TABLE memories ADD COLUMN deprecated_reason TEXT;
ALTER TABLE memories ADD COLUMN supersedes_id TEXT REFERENCES memories(id);

-- Contador de veces que una memoria apareció en contexto inyectado
ALTER TABLE memories ADD COLUMN context_inject_count INTEGER NOT NULL DEFAULT 0;
