-- Where machine bundles live. The bundle's own config.json is the truth about
-- the machine; this table is the index that finds it.
CREATE TABLE IF NOT EXISTS machines (
  id       TEXT PRIMARY KEY NOT NULL,
  path     TEXT NOT NULL UNIQUE,
  name     TEXT NOT NULL,
  os       TEXT NOT NULL,
  family   TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL DEFAULT 0,
  created  INTEGER NOT NULL,
  lastrun  INTEGER
);

-- App preferences, one row per key, values as JSON.
CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);

-- Installers Vista has downloaded and kept, so a second machine of the same
-- system doesn't download it again.
CREATE TABLE IF NOT EXISTS images (
  id      TEXT PRIMARY KEY NOT NULL,
  catalog TEXT NOT NULL,
  name    TEXT NOT NULL,
  version TEXT NOT NULL DEFAULT '',
  url     TEXT NOT NULL,
  path    TEXT NOT NULL,
  size    INTEGER NOT NULL DEFAULT 0,
  sha256  TEXT,
  fetched INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS images_catalog ON images (catalog);
