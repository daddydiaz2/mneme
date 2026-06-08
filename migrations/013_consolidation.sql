-- Memory consolidation: compact stale memories into summaries.
-- A consolidation run picks memories that are old, unused, or deprecated,
-- generates a summary memory, and optionally soft-deletes the originals.

CREATE TABLE IF NOT EXISTS consolidation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    run_at TEXT NOT NULL,            -- RFC3339
    total_consolidated INTEGER NOT NULL DEFAULT 0,
    total_removed INTEGER NOT NULL DEFAULT 0,
    summary_memory_id TEXT,          -- FK to memories(id) with the consolidation summary
    strategy TEXT NOT NULL DEFAULT 'auto',  -- auto, age, deprecated, unused
    rationale TEXT,                  -- why this consolidation happened
    FOREIGN KEY (summary_memory_id) REFERENCES memories(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_consolidation_project ON consolidation_log(project);
CREATE INDEX IF NOT EXISTS idx_consolidation_run ON consolidation_log(run_at);

-- Memory blocks: Letta-inspired slots (human, persona, workflow).
-- Each project can have one memory per slot type.
CREATE TABLE IF NOT EXISTS memory_blocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    slot TEXT NOT NULL,               -- 'human', 'persona', 'workflow'
    memory_id TEXT NOT NULL,           -- FK to memories(id)
    title TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project, slot)
);

CREATE INDEX IF NOT EXISTS idx_memory_blocks_project ON memory_blocks(project);
CREATE INDEX IF NOT EXISTS idx_memory_blocks_slot ON memory_blocks(slot);
