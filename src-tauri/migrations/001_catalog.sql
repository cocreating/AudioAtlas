PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS roots (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  last_scan TEXT
);
CREATE TABLE IF NOT EXISTS files (
  id TEXT PRIMARY KEY,
  root_id TEXT NOT NULL REFERENCES roots(id),
  relative_path TEXT NOT NULL,
  name TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime TEXT NOT NULL,
  format TEXT NOT NULL,
  codec TEXT,
  duration REAL,
  sample_rate INTEGER,
  channels INTEGER,
  bit_depth INTEGER,
  status TEXT NOT NULL DEFAULT 'ready',
  error TEXT,
  UNIQUE(root_id, relative_path)
);
CREATE TABLE IF NOT EXISTS annotations (
  file_id TEXT PRIMARY KEY REFERENCES files(id),
  favorite INTEGER NOT NULL DEFAULT 0,
  tags TEXT NOT NULL DEFAULT '[]',
  notes TEXT NOT NULL DEFAULT ''
);
CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(file_id UNINDEXED, name, path, tags, notes, tokenize='unicode61 remove_diacritics 2');
CREATE INDEX IF NOT EXISTS files_root ON files(root_id, name, id);
CREATE INDEX IF NOT EXISTS files_name ON files(name, id);
CREATE TABLE IF NOT EXISTS exports (
  id TEXT PRIMARY KEY,
  destination TEXT NOT NULL,
  manifest TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
PRAGMA user_version = 1;
