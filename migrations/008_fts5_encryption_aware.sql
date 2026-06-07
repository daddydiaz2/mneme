DROP TRIGGER IF EXISTS memories_ai;
DROP TRIGGER IF EXISTS memories_au;

CREATE TRIGGER IF NOT EXISTS memories_ai
AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, title, content, what, why, context, learned, tags)
    VALUES (
        new.rowid,
        new.title,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.content, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.what, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.why, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.context, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.learned, '') END,
        new.tags
    );
END;

CREATE TRIGGER IF NOT EXISTS memories_au
AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, title, content, what, why, context, learned, tags)
    VALUES (
        'delete', old.rowid,
        old.title, COALESCE(old.content,''), COALESCE(old.what,''),
        COALESCE(old.why,''), COALESCE(old.context,''), COALESCE(old.learned,''), old.tags
    );
    INSERT INTO memories_fts(rowid, title, content, what, why, context, learned, tags)
    VALUES (
        new.rowid,
        new.title,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.content, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.what, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.why, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.context, '') END,
        CASE WHEN new.is_encrypted = 1 THEN '' ELSE COALESCE(new.learned, '') END,
        new.tags
    );
END;
