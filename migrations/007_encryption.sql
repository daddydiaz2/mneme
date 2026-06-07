ALTER TABLE memories ADD COLUMN is_encrypted INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memories ADD COLUMN encrypted_for TEXT;

CREATE TABLE IF NOT EXISTS encryption_keys (
    id          TEXT PRIMARY KEY,
    alias       TEXT NOT NULL,
    key_type    TEXT NOT NULL,
    public_key  TEXT NOT NULL,
    is_default  INTEGER NOT NULL DEFAULT 0,
    added_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_encryption_default ON encryption_keys(is_default);
