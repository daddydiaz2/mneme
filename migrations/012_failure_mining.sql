-- Failure mining: track session outcomes and learn from failures
-- Inspired by Headroom's `headroom learn` and Mem0's MEM0_MIGRATE feedback analysis.

-- Session outcomes: record what happened after a session ended.
-- success = the agent's work was good; failure = the work had issues.
ALTER TABLE sessions ADD COLUMN outcome TEXT;
ALTER TABLE sessions ADD COLUMN failure_reasons TEXT;  -- JSON array: ["wrong_pattern", "missing_context"]
ALTER TABLE sessions ADD COLUMN affected_files INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN bugs_introduced INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN user_corrections TEXT;  -- JSON: what the user had to fix

-- Failure pattern clusters: aggregated signals of recurring issues.
CREATE TABLE IF NOT EXISTS failure_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    pattern_key TEXT NOT NULL,        -- e.g. "missing_context", "outdated_advice"
    description TEXT NOT NULL,
    frequency INTEGER NOT NULL DEFAULT 1,  -- how many sessions hit this
    last_seen TEXT NOT NULL,           -- RFC3339
    first_seen TEXT NOT NULL,          -- RFC3339
    confidence REAL NOT NULL DEFAULT 0.5,
    corrective_memory_id TEXT,         -- FK to memories(id) when generated
    FOREIGN KEY (corrective_memory_id) REFERENCES memories(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_failure_patterns_project ON failure_patterns(project);
CREATE INDEX IF NOT EXISTS idx_failure_patterns_key ON failure_patterns(pattern_key);
CREATE INDEX IF NOT EXISTS idx_failure_patterns_frequency ON failure_patterns(frequency);

-- Auto-generated corrective memories: log of what the learner produced.
CREATE TABLE IF NOT EXISTS corrective_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    failure_pattern_id INTEGER,
    generated_memory_id TEXT,           -- FK to memories(id)
    rationale TEXT,                     -- why this correction was suggested
    generated_at TEXT NOT NULL DEFAULT (datetime('now')),
    user_accepted INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (failure_pattern_id) REFERENCES failure_patterns(id) ON DELETE SET NULL,
    FOREIGN KEY (generated_memory_id) REFERENCES memories(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_corrective_project ON corrective_memories(project);
CREATE INDEX IF NOT EXISTS idx_corrective_pattern ON corrective_memories(failure_pattern_id);
