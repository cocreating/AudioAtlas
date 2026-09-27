PRAGMA foreign_keys = ON;

-- Identidad de volumen y exclusiones configurables para raíces
ALTER TABLE roots ADD COLUMN volume_uuid TEXT;
ALTER TABLE roots ADD COLUMN exclusions TEXT NOT NULL DEFAULT '[]';

-- Valoración (0-5) y estado de revisión del usuario en anotaciones
ALTER TABLE annotations ADD COLUMN rating INTEGER NOT NULL DEFAULT 0;
ALTER TABLE annotations ADD COLUMN status TEXT NOT NULL DEFAULT 'pending';

-- Colecciones manuales ordenadas
CREATE TABLE IF NOT EXISTS collections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  color TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS collection_items (
  collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  file_id TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  position INTEGER NOT NULL DEFAULT 0,
  added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (collection_id, file_id)
);
CREATE INDEX IF NOT EXISTS collection_items_pos ON collection_items(collection_id, position);
CREATE INDEX IF NOT EXISTS collection_items_file ON collection_items(file_id);

-- Consultas inteligentes (filtros guardados)
CREATE TABLE IF NOT EXISTS smart_queries (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  filter_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Estructura para identificación de duplicados por hash completo
CREATE TABLE IF NOT EXISTS contents (
  hash TEXT PRIMARY KEY,
  size INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS file_contents (
  file_id TEXT PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
  hash TEXT NOT NULL REFERENCES contents(hash)
);
CREATE INDEX IF NOT EXISTS file_contents_hash ON file_contents(hash);

PRAGMA user_version = 2;
