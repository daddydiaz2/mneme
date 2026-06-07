-- Temporal validity windows for memory facts
-- Inspired by Graphiti's bi-temporal fact management
-- Each memory can have a validity window: when it became true and when (if ever) it stopped being true

ALTER TABLE memories ADD COLUMN valid_from TEXT;
ALTER TABLE memories ADD COLUMN valid_until TEXT;

-- Index for temporal queries
CREATE INDEX IF NOT EXISTS idx_memories_valid_from ON memories (valid_from);
CREATE INDEX IF NOT EXISTS idx_memories_valid_until ON memories (valid_until);

-- Feedback field for provenance
ALTER TABLE memories ADD COLUMN provenance TEXT;
