-- Tabla FTS5 para búsqueda full-text sobre memorias
-- Nota: SQLite 3.46+ sincroniza automáticamente tablas de contenido externo
-- para INSERT/UPDATE/DELETE. No se requieren triggers.
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    title,
    content,
    what,
    why,
    context,
    learned,
    tags,
    content='memories'
);

-- Tabla FTS5 para búsqueda full-text sobre prompts
CREATE VIRTUAL TABLE IF NOT EXISTS prompts_fts USING fts5(
    content,
    project,
    content='user_prompts'
);
