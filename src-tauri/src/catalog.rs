use rusqlite::{params, Connection, OpenFlags, OptionalExtension, MAIN_DB};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};
use uuid::Uuid;
use walkdir::WalkDir;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone)]
pub struct Catalog {
    path: PathBuf,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: String,
    pub name: String,
    pub path: String,
    pub online: bool,
    pub count: i64,
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub count: i64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SmartQuery {
    pub id: String,
    pub name: String,
    pub filter_json: String,
    pub created_at: String,
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AudioFile {
    pub id: String,
    pub root_id: String,
    pub name: String,
    pub relative_path: String,
    pub size: i64,
    pub format: String,
    pub codec: Option<String>,
    pub duration: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bit_depth: Option<u32>,
    pub status: String,
    pub error: Option<String>,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub notes: String,
    pub rating: u8,
    pub user_status: String,
    pub duplicate_count: u32,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub text: String,
    pub root_id: Option<String>,
    pub favorites: bool,
    pub format: Option<String>,
    pub collection_id: Option<String>,
    pub min_rating: Option<u8>,
    pub duplicates_only: bool,
    pub after: Option<Cursor>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Cursor {
    pub name: String,
    pub id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub roots: Vec<Root>,
    pub files: Vec<AudioFile>,
    pub collections: Vec<Collection>,
    pub smart_queries: Vec<SmartQuery>,
    pub total: i64,
    pub favorites: i64,
    pub duplicates: i64,
    pub matched: i64,
    pub next: Option<Cursor>,
    pub scanning: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateLocation {
    pub file_id: String,
    pub root_id: String,
    pub root_name: String,
    pub relative_path: String,
    pub full_path: String,
    pub size: i64,
}
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSummary {
    pub duplicate_files_count: i64,
    pub duplicate_groups_count: i64,
    pub wasted_bytes: i64,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub favorite: bool,
    pub tags: Vec<String>,
    pub notes: String,
    #[serde(default)]
    pub rating: u8,
    #[serde(default = "default_status")]
    pub status: String,
}
fn default_status() -> String {
    "pending".to_string()
}
impl Default for Annotation {
    fn default() -> Self {
        Self {
            favorite: false,
            tags: Vec::new(),
            notes: String::new(),
            rating: 0,
            status: "pending".to_string(),
        }
    }
}
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub indexed: usize,
    pub errors: usize,
    pub skipped: usize,
    pub done: bool,
    pub canceled: bool,
    pub current: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportItem {
    pub source_id: String,
    pub source_path: String,
    pub destination_name: String,
    pub sha256: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub directory: String,
    pub files: Vec<ExportItem>,
}

impl Catalog {
    pub fn open(path: PathBuf) -> Result<Self> {
        let catalog = Self { path };
        let conn = catalog.connect()?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let mut version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 2 {
            return Err("El catálogo pertenece a una versión más reciente".into());
        }
        if version == 0 {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE;\n{}\nCOMMIT;",
                include_str!("../migrations/001_catalog.sql")
            ))?;
            conn.pragma_update(None, "user_version", 1)?;
            version = 1;
        }
        if version == 1 {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE;\n{}\nCOMMIT;",
                include_str!("../migrations/002_phase1.sql")
            ))?;
            conn.pragma_update(None, "user_version", 2)?;
        }
        Ok(catalog)
    }
    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(conn)
    }
    pub fn add_root(&self, path: &Path) -> Result<String> {
        let path = path.canonicalize()?;
        if !path.is_dir() {
            return Err("Selecciona una carpeta".into());
        }
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id, path FROM roots")?;
        for root in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (id, existing) = root?;
            let existing = Path::new(&existing);
            if existing == path {
                return Ok(id);
            }
            if existing.starts_with(&path) || path.starts_with(existing) {
                return Err("Esta carpeta se solapa con una fuente ya añadida".into());
            }
        }
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO roots(id,path,name) VALUES (?1,?2,?3)",
            params![
                id,
                path.to_str().ok_or("Ruta no válida en UTF-8")?,
                path.file_name().unwrap_or_default().to_string_lossy()
            ],
        )?;
        Ok(id)
    }
    /// Capture a consistent SQLite snapshot, including committed WAL data.
    /// Audio originals are intentionally outside this catalog backup.
    pub fn backup_to(&self, destination: &Path) -> Result<PathBuf> {
        if !destination.is_dir() {
            return Err("Selecciona una carpeta para el respaldo".into());
        }
        let mut temp = tempfile::NamedTempFile::new_in(destination)?;
        self.connect()?.backup(MAIN_DB, temp.path(), None)?;
        validate_catalog_snapshot(temp.path())?;
        temp.as_file_mut().sync_all()?;
        let path = destination.join(format!("AudioAtlas-catalog-{}.sqlite", Uuid::new_v4()));
        temp.persist_noclobber(&path)?;
        Ok(path)
    }
    /// Stage a validated, consistent snapshot for the next application launch.
    pub fn stage_restore(source: &Path, data_dir: &Path) -> Result<()> {
        let pending = data_dir.join("catalog-restore-pending.sqlite");
        if pending.exists() {
            return Err("Ya hay una restauración preparada; reinicia la aplicación".into());
        }
        validate_catalog_snapshot(source)?;
        let temp = tempfile::NamedTempFile::new_in(data_dir)?;
        Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?.backup(
            MAIN_DB,
            temp.path(),
            None,
        )?;
        validate_catalog_snapshot(temp.path())?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(pending)?;
        Ok(())
    }
    /// Run before opening the app's catalog. Keep a recovery snapshot of the
    /// previous catalog because restore replaces its organization in place.
    pub fn apply_pending_restore(data_dir: &Path) -> Result<Option<PathBuf>> {
        let pending = data_dir.join("catalog-restore-pending.sqlite");
        if !pending.exists() {
            return Ok(None);
        }
        validate_catalog_snapshot(&pending)?;
        let current = data_dir.join("catalog.sqlite");
        let recovery = if current.exists() {
            let path = data_dir.join(format!("catalog-before-restore-{}.sqlite", Uuid::new_v4()));
            Connection::open(&current)?.backup(MAIN_DB, &path, None)?;
            Some(path)
        } else {
            None
        };
        let applied = (|| -> Result<()> {
            let mut destination = Connection::open(&current)?;
            destination.restore(MAIN_DB, &pending, None::<fn(rusqlite::backup::Progress)>)?;
            drop(destination);
            validate_catalog_snapshot(&current)
        })();
        if let Err(error) = applied {
            if let Some(previous) = &recovery {
                let mut destination = Connection::open(&current)?;
                destination.restore(MAIN_DB, previous, None::<fn(rusqlite::backup::Progress)>)?;
            }
            return Err(error);
        }
        fs::remove_file(pending)?;
        Ok(recovery)
    }
    fn root_path(&self, id: &str) -> Result<PathBuf> {
        Ok(PathBuf::from(self.connect()?.query_row(
            "SELECT path FROM roots WHERE id=?1",
            [id],
            |r| r.get::<_, String>(0),
        )?))
    }
    pub fn resolve(&self, id: &str) -> Result<PathBuf> {
        let (root, relative): (String,String) = self.connect()?.query_row("SELECT r.path,f.relative_path FROM files f JOIN roots r ON r.id=f.root_id WHERE f.id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?)))?;
        let root = Path::new(&root).canonicalize()?;
        let path = root.join(relative).canonicalize()?;
        if !path.starts_with(&root) || !path.is_file() {
            return Err("El archivo no está dentro de una fuente autorizada".into());
        }
        Ok(path)
    }
    pub fn scan(
        &self,
        root_id: &str,
        cancel: &AtomicBool,
        mut progress: impl FnMut(ScanProgress),
    ) -> Result<ScanProgress> {
        let root = self.root_path(root_id)?;
        if !root.is_dir() {
            return Err("Fuente desconectada; el catálogo y las etiquetas se conservan".into());
        }
        let mut report = ScanProgress::default();
        let mut conn = self.connect()?;
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        {
            if cancel.load(Ordering::Relaxed) {
                report.canceled = true;
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    report.errors += 1;
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            // macOS UF_DATALESS: never hydrate iCloud placeholders as a side effect.
            #[cfg(target_os = "macos")]
            {
                use std::os::macos::fs::MetadataExt;
                if entry
                    .metadata()
                    .map(|m| m.st_flags() & 0x40000000 != 0)
                    .unwrap_or(true)
                {
                    report.skipped += 1;
                    continue;
                }
            }
            let ext = entry
                .path()
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if ![
                "wav", "wave", "aif", "aiff", "flac", "mp3", "m4a", "aac", "ogg", "oga",
            ]
            .contains(&ext.as_str())
            {
                continue;
            }
            report.current = entry.file_name().to_string_lossy().into_owned();
            let result = (|| -> Result<()> {
                let relative = entry
                    .path()
                    .strip_prefix(&root)?
                    .to_str()
                    .ok_or("Ruta no UTF-8")?
                    .to_string();
                let before = stamp(entry.path())?;
                let previous: Option<(String, i64, String)> = conn
                    .query_row(
                        "SELECT id,size,mtime FROM files WHERE root_id=?1 AND relative_path=?2",
                        params![root_id, relative],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()?;
                if previous
                    .as_ref()
                    .is_some_and(|(_, size, time)| *size == before.0 && *time == before.1)
                {
                    conn.execute("UPDATE files SET status=CASE WHEN error IS NULL THEN 'ready' ELSE 'unsupported' END WHERE root_id=?1 AND relative_path=?2", params![root_id,relative])?;
                    return Ok(());
                }
                // The old digest no longer describes this path, even if probing fails.
                if let Some((id, _, _)) = &previous {
                    conn.execute("DELETE FROM file_contents WHERE file_id=?1", [id])?;
                }
                let metadata = probe(entry.path());
                if stamp(entry.path())? != before {
                    return Err("El archivo cambió durante la lectura; vuelve a escanear".into());
                }
                let id = previous
                    .map(|x| x.0)
                    .unwrap_or_else(|| Uuid::new_v4().to_string());
                let (duration, sample_rate, channels, bit_depth, codec, error) = match metadata {
                    Ok(m) => (
                        m.duration,
                        m.sample_rate,
                        m.channels,
                        m.bit_depth,
                        Some(m.codec),
                        None,
                    ),
                    Err(e) => {
                        report.errors += 1;
                        (None, None, None, None, None, Some(e.to_string()))
                    }
                };
                let tx = conn.transaction()?;
                tx.execute("INSERT INTO files (id,root_id,relative_path,name,size,mtime,format,codec,duration,sample_rate,channels,bit_depth,status,error) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14) ON CONFLICT(root_id,relative_path) DO UPDATE SET size=excluded.size,mtime=excluded.mtime,format=excluded.format,codec=excluded.codec,duration=excluded.duration,sample_rate=excluded.sample_rate,channels=excluded.channels,bit_depth=excluded.bit_depth,status=excluded.status,error=excluded.error", params![id,root_id,relative,report.current,before.0,before.1,ext.to_uppercase(),codec,duration,sample_rate,channels,bit_depth,if error.is_some() {"unsupported"} else {"ready"},error])?;
                tx.execute(
                    "INSERT OR IGNORE INTO annotations(file_id) VALUES (?1)",
                    [&id],
                )?;
                refresh_search(&tx, &id)?;
                tx.commit()?;
                Ok(())
            })();
            if result.is_err() {
                report.errors += 1;
            }
            report.indexed += 1;
            if report.indexed % 10 == 0 {
                progress(report.clone());
            }
        }
        if !report.canceled && root.is_dir() {
            // Only a completed scan may mark missing files. Never erase their annotations.
            let mut stmt = conn.prepare("SELECT id,relative_path FROM files WHERE root_id=?1")?;
            let known = stmt
                .query_map([root_id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            drop(stmt);
            for (id, path) in known {
                if !root.join(path).exists() {
                    let tx = conn.transaction()?;
                    tx.execute("DELETE FROM file_contents WHERE file_id=?1", [&id])?;
                    tx.execute("UPDATE files SET status='missing' WHERE id=?1", [&id])?;
                    tx.commit()?;
                }
            }
            conn.execute(
                "UPDATE roots SET last_scan=CURRENT_TIMESTAMP WHERE id=?1",
                [root_id],
            )?;
        }
        if !report.canceled {
            self.index_duplicates()?;
        }
        report.done = true;
        progress(report.clone());
        Ok(report)
    }
    pub fn query(&self, query: Query) -> Result<Library> {
        let conn = self.connect()?;
        let roots = conn.prepare("SELECT r.id,r.name,r.path,COUNT(f.id) FROM roots r LEFT JOIN files f ON f.root_id=r.id GROUP BY r.id ORDER BY r.name")?.query_map([], |r| { let path: String = r.get(2)?; Ok(Root {id:r.get(0)?,name:r.get(1)?,online:Path::new(&path).is_dir(),path,count:r.get(3)?}) })?.collect::<std::result::Result<Vec<_>,_>>()?;
        let collections = self.list_collections_with_conn(&conn)?;
        let smart_queries = self.list_smart_queries_with_conn(&conn)?;
        let total = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let favorites = conn.query_row(
            "SELECT COUNT(*) FROM annotations WHERE favorite=1",
            [],
            |r| r.get(0),
        )?;
        let duplicates: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT fc.file_id) \
                 FROM file_contents fc \
                 JOIN (SELECT hash FROM file_contents GROUP BY hash HAVING COUNT(*) > 1) d \
                   ON d.hash = fc.hash",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let search = query
            .text
            .split_whitespace()
            .take(20)
            .map(|s| format!("\"{}\"*", s.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let search = if search.is_empty() {
            None
        } else {
            Some(search)
        };
        let base = "FROM files f JOIN annotations a ON a.file_id=f.id WHERE (?1 IS NULL OR f.id IN (SELECT file_id FROM search WHERE search MATCH ?1)) AND (?2 IS NULL OR f.root_id=?2) AND (?3=0 OR a.favorite=1) AND (?4 IS NULL OR f.format=?4) AND (?5 IS NULL OR f.id IN (SELECT file_id FROM collection_items WHERE collection_id=?5)) AND (?6 IS NULL OR a.rating>=?6) AND (?7=0 OR f.id IN (SELECT fc.file_id FROM file_contents fc JOIN (SELECT hash FROM file_contents GROUP BY hash HAVING COUNT(*) > 1) d ON d.hash = fc.hash))";
        let matched = conn.query_row(
            &format!("SELECT COUNT(*) {base}"),
            params![
                search,
                query.root_id,
                query.favorites,
                query.format,
                query.collection_id,
                query.min_rating,
                query.duplicates_only
            ],
            |r| r.get(0),
        )?;
        let sql = format!(
            "SELECT f.id,f.root_id,f.name,f.relative_path,f.size,f.format,f.codec,f.duration,f.sample_rate,\
             f.channels,f.bit_depth,f.status,f.error,a.favorite,a.tags,a.notes,a.rating,a.status,\
             MAX(0, COALESCE((SELECT COUNT(*) - 1 FROM file_contents fc2 WHERE fc2.hash = (SELECT hash FROM file_contents fc WHERE fc.file_id = f.id)), 0)) \
             {base} AND (?8 IS NULL OR (f.name,f.id)>(?8,?9)) ORDER BY f.name,f.id LIMIT 101"
        );
        let mut files = conn
            .prepare(&sql)?
            .query_map(
                params![
                    search,
                    query.root_id,
                    query.favorites,
                    query.format,
                    query.collection_id,
                    query.min_rating,
                    query.duplicates_only,
                    query.after.as_ref().map(|x| &x.name),
                    query.after.as_ref().map(|x| &x.id)
                ],
                |r| {
                    Ok(AudioFile {
                        id: r.get(0)?,
                        root_id: r.get(1)?,
                        name: r.get(2)?,
                        relative_path: r.get(3)?,
                        size: r.get(4)?,
                        format: r.get(5)?,
                        codec: r.get(6)?,
                        duration: r.get(7)?,
                        sample_rate: r.get(8)?,
                        channels: r.get(9)?,
                        bit_depth: r.get(10)?,
                        status: r.get(11)?,
                        error: r.get(12)?,
                        favorite: r.get(13)?,
                        tags: serde_json::from_str(&r.get::<_, String>(14)?).unwrap_or_default(),
                        notes: r.get(15)?,
                        rating: r.get(16)?,
                        user_status: r.get(17)?,
                        duplicate_count: r.get::<_, i64>(18)? as u32,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let next = if files.len() > 100 {
            files.pop();
            files.last().map(|f| Cursor {
                name: f.name.clone(),
                id: f.id.clone(),
            })
        } else {
            None
        };
        for file in &mut files {
            if roots.iter().any(|r| r.id == file.root_id && !r.online) {
                file.status = "offline".into();
            }
        }
        Ok(Library {
            roots,
            files,
            collections,
            smart_queries,
            total,
            favorites,
            duplicates,
            matched,
            next,
            scanning: false,
        })
    }
    pub fn annotate(&self, id: &str, mut annotation: Annotation) -> Result<()> {
        if annotation.notes.len() > 20000 || annotation.tags.len() > 100 {
            return Err("Las anotaciones superan el tamaño permitido".into());
        }
        if annotation.rating > 5 {
            return Err("La valoración debe estar entre 0 y 5".into());
        }
        annotation.tags = annotation
            .tags
            .into_iter()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        if annotation.tags.iter().any(|x| x.len() > 100) {
            return Err("Cada etiqueta admite hasta 100 bytes".into());
        }
        annotation.tags.sort();
        annotation.tags.dedup();
        let status = if annotation.status.trim().is_empty() {
            "pending".to_string()
        } else {
            annotation.status.trim().to_string()
        };
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        let count = tx.execute(
            "UPDATE annotations SET favorite=?1,tags=?2,notes=?3,rating=?4,status=?5 WHERE file_id=?6",
            params![
                annotation.favorite,
                serde_json::to_string(&annotation.tags)?,
                annotation.notes,
                annotation.rating,
                status,
                id
            ],
        )?;
        if count == 0 {
            return Err("Archivo no encontrado en el catálogo".into());
        }
        refresh_search(&tx, id)?;
        tx.commit()?;
        Ok(())
    }
    fn list_collections_with_conn(&self, conn: &Connection) -> Result<Vec<Collection>> {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.name, c.color, COUNT(ci.file_id), c.created_at, c.updated_at \
             FROM collections c \
             LEFT JOIN collection_items ci ON ci.collection_id = c.id \
             GROUP BY c.id \
             ORDER BY c.name COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Collection {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                count: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
    pub fn list_collections(&self) -> Result<Vec<Collection>> {
        let conn = self.connect()?;
        self.list_collections_with_conn(&conn)
    }
    pub fn create_collection(&self, name: &str, color: Option<&str>) -> Result<Collection> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El nombre de la colección no puede estar vacío".into());
        }
        if name.len() > 100 {
            return Err("El nombre de la colección no puede exceder 100 caracteres".into());
        }
        let id = Uuid::new_v4().to_string();
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO collections (id, name, color) VALUES (?1, ?2, ?3)",
            params![id, name, color],
        )?;
        let (created_at, updated_at): (String, String) = conn.query_row(
            "SELECT created_at, updated_at FROM collections WHERE id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(Collection {
            id,
            name: name.to_string(),
            color: color.map(|s| s.to_string()),
            count: 0,
            created_at,
            updated_at,
        })
    }
    pub fn rename_collection(&self, id: &str, name: &str, color: Option<&str>) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El nombre de la colección no puede estar vacío".into());
        }
        if name.len() > 100 {
            return Err("El nombre de la colección no puede exceder 100 caracteres".into());
        }
        let conn = self.connect()?;
        let count = conn.execute(
            "UPDATE collections SET name=?1, color=COALESCE(?2, color), updated_at=CURRENT_TIMESTAMP WHERE id=?3",
            params![name, color, id],
        )?;
        if count == 0 {
            return Err("Colección no encontrada".into());
        }
        Ok(())
    }
    pub fn delete_collection(&self, id: &str) -> Result<()> {
        let conn = self.connect()?;
        let count = conn.execute("DELETE FROM collections WHERE id=?1", [id])?;
        if count == 0 {
            return Err("Colección no encontrada".into());
        }
        Ok(())
    }
    pub fn add_to_collection(&self, collection_id: &str, file_ids: &[String]) -> Result<usize> {
        if file_ids.is_empty() {
            return Ok(0);
        }
        let mut conn = self.connect()?;
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM collections WHERE id=?1",
                [collection_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !exists {
            return Err("Colección no encontrada".into());
        }
        let tx = conn.transaction()?;
        let mut added = 0;
        let mut next_pos: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM collection_items WHERE collection_id=?1",
            [collection_id],
            |r| r.get(0),
        )?;
        for file_id in file_ids {
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO collection_items (collection_id, file_id, position) VALUES (?1, ?2, ?3)",
                params![collection_id, file_id, next_pos],
            )?;
            if inserted > 0 {
                added += 1;
                next_pos += 1;
            }
        }
        tx.execute(
            "UPDATE collections SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [collection_id],
        )?;
        tx.commit()?;
        Ok(added)
    }
    pub fn remove_from_collection(&self, collection_id: &str, file_ids: &[String]) -> Result<()> {
        if file_ids.is_empty() {
            return Ok(());
        }
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        for file_id in file_ids {
            tx.execute(
                "DELETE FROM collection_items WHERE collection_id=?1 AND file_id=?2",
                params![collection_id, file_id],
            )?;
        }
        tx.execute(
            "UPDATE collections SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [collection_id],
        )?;
        tx.commit()?;
        Ok(())
    }
    fn list_smart_queries_with_conn(&self, conn: &Connection) -> Result<Vec<SmartQuery>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, filter_json, created_at FROM smart_queries ORDER BY name COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SmartQuery {
                id: r.get(0)?,
                name: r.get(1)?,
                filter_json: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
    pub fn list_smart_queries(&self) -> Result<Vec<SmartQuery>> {
        let conn = self.connect()?;
        self.list_smart_queries_with_conn(&conn)
    }
    pub fn save_smart_query(&self, name: &str, filter_json: &str) -> Result<SmartQuery> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El nombre del filtro no puede estar vacío".into());
        }
        if name.len() > 100 {
            return Err("El nombre del filtro no puede exceder 100 caracteres".into());
        }
        let _parsed: serde_json::Value = serde_json::from_str(filter_json)
            .map_err(|_| "El filtro guardado debe ser un JSON válido")?;
        let id = Uuid::new_v4().to_string();
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO smart_queries (id, name, filter_json) VALUES (?1, ?2, ?3)",
            params![id, name, filter_json],
        )?;
        let created_at: String = conn.query_row(
            "SELECT created_at FROM smart_queries WHERE id=?1",
            [&id],
            |r| r.get(0),
        )?;
        Ok(SmartQuery {
            id,
            name: name.to_string(),
            filter_json: filter_json.to_string(),
            created_at,
        })
    }
    pub fn delete_smart_query(&self, id: &str) -> Result<()> {
        let conn = self.connect()?;
        let count = conn.execute("DELETE FROM smart_queries WHERE id=?1", [id])?;
        if count == 0 {
            return Err("Filtro guardado no encontrado".into());
        }
        Ok(())
    }
    pub fn index_duplicates(&self) -> Result<usize> {
        let mut conn = self.connect()?;
        // Reconcile hashes made by an earlier scan, including files changed while
        // the application was closed or removed from an offline source.
        let mut linked_stmt = conn.prepare(
            "SELECT f.id,f.size,f.mtime FROM file_contents fc JOIN files f ON f.id=fc.file_id",
        )?;
        let linked = linked_stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let invalid = linked
            .into_iter()
            .filter_map(|(id, size, mtime)| {
                let valid = self
                    .resolve(&id)
                    .and_then(|path| stamp(&path))
                    .is_ok_and(|current| current == (size, mtime));
                (!valid).then_some(id)
            })
            .collect::<Vec<_>>();
        drop(linked_stmt);
        if !invalid.is_empty() {
            let tx = conn.transaction()?;
            for id in invalid {
                tx.execute("DELETE FROM file_contents WHERE file_id=?1", [id])?;
            }
            tx.commit()?;
        }
        let mut stmt = conn.prepare(
            "SELECT f.id, f.size, f.mtime \
             FROM files f \
             WHERE f.status IN ('ready','unsupported') \
               AND f.size > 0 \
               AND f.size IN ( \
                   SELECT size FROM files WHERE status IN ('ready','unsupported') AND size > 0 GROUP BY size HAVING COUNT(*) > 1 \
               ) \
               AND f.id NOT IN (SELECT file_id FROM file_contents)",
        )?;
        let candidates = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);

        if candidates.is_empty() {
            return Ok(0);
        }

        let mut hashes = Vec::new();
        for (file_id, size, mtime) in candidates {
            let Ok(full_path) = self.resolve(&file_id) else {
                continue;
            };
            if stamp(&full_path).ok() != Some((size, mtime.clone())) {
                continue;
            }
            let digest = match hash_file(&full_path) {
                Ok(h) => h,
                Err(_) => continue,
            };
            if stamp(&full_path).ok() != Some((size, mtime)) {
                continue;
            }
            hashes.push((file_id, digest, size));
        }
        let tx = conn.transaction()?;
        for (file_id, digest, size) in &hashes {
            tx.execute(
                "INSERT OR IGNORE INTO contents (hash, size) VALUES (?1, ?2)",
                params![digest, size],
            )?;

            tx.execute(
                "INSERT INTO file_contents (file_id, hash) VALUES (?1, ?2) \
                 ON CONFLICT(file_id) DO UPDATE SET hash = excluded.hash",
                params![file_id, digest],
            )?;
        }
        tx.commit()?;
        Ok(hashes.len())
    }
    pub fn duplicate_summary(&self) -> Result<DuplicateSummary> {
        let conn = self.connect()?;
        let (files_count, groups_count, wasted_bytes): (i64, i64, i64) = conn
            .query_row(
                "SELECT \
                    COALESCE((SELECT COUNT(fc.file_id) FROM file_contents fc WHERE fc.hash IN (SELECT hash FROM file_contents GROUP BY hash HAVING COUNT(*) > 1)), 0), \
                    COALESCE((SELECT COUNT(*) FROM (SELECT hash FROM file_contents GROUP BY hash HAVING COUNT(*) > 1)), 0), \
                    COALESCE((SELECT SUM(c.size * (cnt - 1)) FROM (SELECT hash, COUNT(*) as cnt FROM file_contents GROUP BY hash HAVING cnt > 1) dup JOIN contents c ON c.hash = dup.hash), 0)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap_or((0, 0, 0));

        Ok(DuplicateSummary {
            duplicate_files_count: files_count,
            duplicate_groups_count: groups_count,
            wasted_bytes,
        })
    }
    pub fn get_file_duplicates(&self, file_id: &str) -> Result<Vec<DuplicateLocation>> {
        let mut conn = self.connect()?;
        let has_content: bool = conn
            .query_row(
                "SELECT 1 FROM file_contents WHERE file_id = ?1",
                [file_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_content {
            drop(conn);
            let _ = self.index_duplicates();
            conn = self.connect()?;
        }

        let mut stmt = conn.prepare(
            "SELECT f.id, f.root_id, r.name, f.relative_path, r.path, f.size \
             FROM file_contents fc \
             JOIN file_contents fc_target ON fc_target.hash = fc.hash \
             JOIN files f ON f.id = fc.file_id \
             JOIN roots r ON r.id = f.root_id \
             WHERE fc_target.file_id = ?1 AND f.id != ?1 \
             ORDER BY r.name COLLATE NOCASE, f.relative_path COLLATE NOCASE",
        )?;

        let rows = stmt.query_map([file_id], |r| {
            let root_path: String = r.get(4)?;
            let rel_path: String = r.get(3)?;
            let full_path = Path::new(&root_path)
                .join(&rel_path)
                .to_string_lossy()
                .into_owned();
            Ok(DuplicateLocation {
                file_id: r.get(0)?,
                root_id: r.get(1)?,
                root_name: r.get(2)?,
                relative_path: rel_path,
                full_path,
                size: r.get(5)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
    pub fn export(&self, ids: &[String], destination: &Path) -> Result<ExportResult> {
        if ids.is_empty() || ids.len() > 1000 {
            return Err("Selecciona entre 1 y 1000 archivos".into());
        }
        let destination = destination.canonicalize()?;
        // A unique durable session folder prevents collisions with originals and other exports.
        let id = Uuid::new_v4().to_string();
        let directory = destination.join(format!("Audio Atlas {}", &id[..8]));
        fs::create_dir(&directory)?;
        let mut result = ExportResult {
            directory: directory.to_string_lossy().into_owned(),
            files: Vec::new(),
        };
        for source_id in ids {
            let copied = (|| -> Result<ExportItem> {
                let source = self.resolve(source_id)?;
                let before = stamp(&source)?;
                let filename = source
                    .file_name()
                    .ok_or("Nombre no válido")?
                    .to_string_lossy();
                let name = format!("{}-{}", &source_id[..8], filename);
                let mut input = File::open(&source)?;
                let mut output = tempfile::NamedTempFile::new_in(&directory)?;
                let mut hash = Sha256::new();
                let mut buffer = [0u8; 65536];
                loop {
                    let n = input.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    hash.update(&buffer[..n]);
                    output.write_all(&buffer[..n])?;
                }
                output.as_file().sync_all()?;
                let digest = format!("{:x}", hash.finalize());
                if stamp(&source)? != before || digest != hash_file(output.path())? {
                    return Err("El archivo cambió o la copia no se pudo verificar".into());
                }
                output.persist_noclobber(directory.join(&name))?;
                Ok(ExportItem {
                    source_id: source_id.clone(),
                    source_path: source.to_string_lossy().into_owned(),
                    destination_name: name,
                    sha256: digest,
                })
            })();
            match copied {
                Ok(item) => result.files.push(item),
                Err(error) => {
                    fs::write(
                        directory.join("export-error.json"),
                        serde_json::to_vec_pretty(
                            &serde_json::json!({"error":error.to_string(),"completed":result.files}),
                        )?,
                    )?;
                    return Err(
                        format!("Exportación parcial en {}: {error}", directory.display()).into(),
                    );
                }
            }
        }
        let manifest = serde_json::to_string_pretty(
            &serde_json::json!({"version":1,"transforms":[],"files":result.files}),
        )?;
        let mut manifest_file = tempfile::NamedTempFile::new_in(&directory)?;
        manifest_file.write_all(manifest.as_bytes())?;
        manifest_file.as_file().sync_all()?;
        manifest_file.persist_noclobber(directory.join("manifest.json"))?;
        self.connect()?.execute(
            "INSERT INTO exports(id,destination,manifest) VALUES (?1,?2,?3)",
            params![id, result.directory, manifest],
        )?;
        Ok(result)
    }
}
fn refresh_search(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM search WHERE file_id=?1", [id])?;
    conn.execute("INSERT INTO search(file_id,name,path,tags,notes) SELECT f.id,f.name,f.relative_path,a.tags,a.notes FROM files f JOIN annotations a ON a.file_id=f.id WHERE f.id=?1",[id])?;
    Ok(())
}
fn validate_catalog_snapshot(path: &Path) -> Result<()> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let tables: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view') AND name IN ('roots','files','annotations','collections','collection_items','smart_queries','contents','file_contents','search')",
        [],
        |r| r.get(0),
    )?;
    let broken_references: i64 =
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    if integrity != "ok" || version != 2 || tables != 9 || broken_references != 0 {
        return Err("El archivo no es un respaldo válido del catálogo actual".into());
    }
    Ok(())
}
fn stamp(path: &Path) -> Result<(i64, String)> {
    let m = fs::metadata(path)?;
    Ok((
        m.len() as i64,
        m.modified()?
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string(),
    ))
}
pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
struct Metadata {
    duration: Option<f64>,
    sample_rate: Option<u32>,
    channels: Option<u32>,
    bit_depth: Option<u32>,
    codec: String,
}
fn probe(path: &Path) -> Result<Metadata> {
    let source = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe().format(
        &hint,
        source,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let params = &probed
        .format
        .default_track()
        .ok_or("No hay pista de audio compatible")?
        .codec_params;
    symphonia::default::get_codecs().make(params, &Default::default())?;
    let duration = params.n_frames.zip(params.time_base).map(|(frames, base)| {
        let t = base.calc_time(frames);
        t.seconds as f64 + t.frac
    });
    let codec = symphonia::default::get_codecs()
        .get_codec(params.codec)
        .map(|c| c.short_name.to_string())
        .unwrap_or_else(|| "desconocido".into());
    Ok(Metadata {
        duration,
        sample_rate: params.sample_rate,
        channels: params.channels.map(|c| c.count() as u32),
        bit_depth: params.bits_per_sample,
        codec,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wav(path: &Path) {
        wav_value(path, 0);
    }
    fn wav_value(path: &Path, value: i16) {
        let mut w = hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..44100 {
            w.write_sample(value).unwrap();
        }
        w.finalize().unwrap();
    }
    #[test]
    fn vertical_persists_searches_and_exports_without_touching_originals() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sonidos ñ");
        fs::create_dir(&root).unwrap();
        let source = root.join("ambiente.wav");
        wav(&source);
        let before = hash_file(&source).unwrap();
        let db = temp.path().join("catalog.sqlite");
        let catalog = Catalog::open(db.clone()).unwrap();
        let root_id = catalog.add_root(&root).unwrap();
        fs::write(root.join("corrupto.wav"), "not audio").unwrap();
        let scan = catalog
            .scan(&root_id, &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(scan.indexed, 2);
        assert_eq!(scan.errors, 1);
        let rows = catalog.query(Query::default()).unwrap();
        let file = rows
            .files
            .iter()
            .find(|f| f.name == "ambiente.wav")
            .unwrap();
        assert_eq!(file.sample_rate, Some(44100));
        assert_eq!(file.duration, Some(1.0));
        let id = file.id.clone();
        catalog
            .annotate(
                &id,
                Annotation {
                    favorite: true,
                    tags: vec!["atmósfera".into()],
                    notes: "bosque".into(),
                    rating: 5,
                    status: "listened".into(),
                },
            )
            .unwrap();
        drop(catalog);
        let catalog = Catalog::open(db).unwrap();
        let found = catalog
            .query(Query {
                text: "atmosfera bosque".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(found.files.len(), 1);
        assert!(found.files[0].favorite);
        assert_eq!(found.files[0].rating, 5);
        assert_eq!(found.files[0].user_status, "listened");
        catalog
            .scan(&root_id, &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(catalog.query(Query::default()).unwrap().total, 2);
        let out = catalog
            .export(std::slice::from_ref(&id), temp.path())
            .unwrap();
        assert_eq!(hash_file(&source).unwrap(), before);
        assert_eq!(
            hash_file(&Path::new(&out.directory).join(&out.files[0].destination_name)).unwrap(),
            before
        );
        let out2 = catalog.export(&[id], temp.path()).unwrap();
        assert_ne!(out.directory, out2.directory);
        fs::rename(&root, temp.path().join("offline")).unwrap();
        let offline = catalog
            .query(Query {
                favorites: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(offline.files[0].status, "offline");
        assert_eq!(offline.files[0].tags, vec!["atmósfera"]);
    }
    #[test]
    fn rejects_overlapping_roots_and_escape_via_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let source = root.join("a.wav");
        wav(&source);
        let catalog = Catalog::open(temp.path().join("db")).unwrap();
        let id = catalog.add_root(&root).unwrap();
        assert!(catalog.add_root(temp.path()).is_err());
        catalog.scan(&id, &AtomicBool::new(false), |_| {}).unwrap();
        let file_id = catalog.query(Query::default()).unwrap().files[0].id.clone();
        #[cfg(unix)]
        {
            let external = temp.path().join("outside.wav");
            fs::rename(&source, &external).unwrap();
            std::os::unix::fs::symlink(external, &source).unwrap();
            assert!(catalog.resolve(&file_id).is_err());
        }
    }
    #[test]
    fn cursor_pagination_and_search_syntax_are_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        for n in 0..105 {
            wav(&root.join(format!("sound-{n:03}.wav")));
        }
        let catalog = Catalog::open(temp.path().join("db")).unwrap();
        let id = catalog.add_root(&root).unwrap();
        catalog.scan(&id, &AtomicBool::new(false), |_| {}).unwrap();
        let first = catalog.query(Query::default()).unwrap();
        assert_eq!(first.files.len(), 100);
        let second = catalog
            .query(Query {
                after: first.next,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(second.files.len(), 5);
        assert!(second.next.is_none());
        assert!(catalog
            .query(Query {
                text: "\" OR * : -".into(),
                ..Default::default()
            })
            .is_ok());
    }
    #[test]
    fn test_migration_v1_to_v2_and_collections_smart_queries() {
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("v1_to_v2.sqlite");

        // 1. Inicializa manualmente en v1 con 001_catalog.sql
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch(include_str!("../migrations/001_catalog.sql"))
                .unwrap();
            let v: i64 = conn
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .unwrap();
            assert_eq!(v, 1);
            conn.execute(
                "INSERT INTO roots (id, path, name) VALUES ('r1', '/tmp/fake', 'fake')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO files (id, root_id, relative_path, name, size, mtime, format, status) VALUES ('f1', 'r1', 'a.wav', 'a.wav', 100, '123', 'WAV', 'ready')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO annotations (file_id, favorite, tags, notes) VALUES ('f1', 1, '[\"kick\"]', 'good')",
                [],
            )
            .unwrap();
        }

        // 2. Abre con Catalog::open, aplicando migración 002
        let catalog = Catalog::open(db.clone()).unwrap();
        {
            let conn = catalog.connect().unwrap();
            let v: i64 = conn
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .unwrap();
            assert_eq!(v, 2);
        }

        // 3. Verifica valores por defecto en registros migrados
        let lib = catalog.query(Query::default()).unwrap();
        assert_eq!(lib.files.len(), 1);
        assert_eq!(lib.files[0].rating, 0);
        assert_eq!(lib.files[0].user_status, "pending");

        // 4. Actualiza anotación con rating y status
        catalog
            .annotate(
                "f1",
                Annotation {
                    favorite: true,
                    tags: vec!["kick".into(), "punchy".into()],
                    notes: "tested".into(),
                    rating: 4,
                    status: "listened".into(),
                },
            )
            .unwrap();

        // 5. Consulta por min_rating
        let rated_match = catalog
            .query(Query {
                min_rating: Some(4),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rated_match.files.len(), 1);
        assert_eq!(rated_match.files[0].rating, 4);

        let unrated_match = catalog
            .query(Query {
                min_rating: Some(5),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(unrated_match.files.len(), 0);

        // 6. Prueba Colecciones
        let col = catalog
            .create_collection("Favoritos Live", Some("#ff5500"))
            .unwrap();
        assert_eq!(col.name, "Favoritos Live");
        assert_eq!(col.count, 0);

        let added = catalog
            .add_to_collection(&col.id, &["f1".to_string()])
            .unwrap();
        assert_eq!(added, 1);

        let cols = catalog.list_collections().unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].count, 1);

        let in_col = catalog
            .query(Query {
                collection_id: Some(col.id.clone()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(in_col.files.len(), 1);
        assert_eq!(in_col.files[0].id, "f1");

        // 7. Prueba Smart Queries
        let sq = catalog
            .save_smart_query("Kicks 4+ estrellas", r#"{"minRating":4,"text":"kick"}"#)
            .unwrap();
        assert_eq!(sq.name, "Kicks 4+ estrellas");

        let sqs = catalog.list_smart_queries().unwrap();
        assert_eq!(sqs.len(), 1);
        assert_eq!(sqs[0].id, sq.id);

        // 8. Quitar de colección
        catalog
            .remove_from_collection(&col.id, &["f1".to_string()])
            .unwrap();
        let cols_after = catalog.list_collections().unwrap();
        assert_eq!(cols_after[0].count, 0);

        // 9. Borrar colección y smart query
        catalog.delete_collection(&col.id).unwrap();
        assert_eq!(catalog.list_collections().unwrap().len(), 0);

        catalog.delete_smart_query(&sq.id).unwrap();
        assert_eq!(catalog.list_smart_queries().unwrap().len(), 0);
    }

    #[test]
    fn test_exact_duplicate_detection_by_streaming_hash() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("samples");
        fs::create_dir_all(root.join("subfolder")).unwrap();

        // 1. Create original kick and an exact duplicate in subfolder
        let kick1 = root.join("kick_orig.wav");
        wav(&kick1);
        let kick2 = root.join("subfolder").join("kick_copy.wav");
        fs::copy(&kick1, &kick2).unwrap();

        // 2. Create another sound (snare) with different duration/samples
        let snare = root.join("snare.wav");
        let mut w = hound::WavWriter::create(
            &snare,
            hound::WavSpec {
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..22050 {
            w.write_sample(500i16).unwrap();
        }
        w.finalize().unwrap();

        let db = temp.path().join("catalog.sqlite");
        let catalog = Catalog::open(db).unwrap();
        let root_id = catalog.add_root(&root).unwrap();

        let scan = catalog
            .scan(&root_id, &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(scan.indexed, 3);
        assert_eq!(scan.errors, 0);

        // Scan automatically indexes duplicates via streaming SHA-256
        let summary = catalog.duplicate_summary().unwrap();
        assert_eq!(summary.duplicate_files_count, 2);
        assert_eq!(summary.duplicate_groups_count, 1);
        assert_eq!(
            summary.wasted_bytes,
            fs::metadata(&kick1).unwrap().len() as i64
        );

        // Query duplicates only
        let dup_view = catalog
            .query(Query {
                duplicates_only: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(dup_view.matched, 2);
        assert_eq!(dup_view.files.len(), 2);
        assert_eq!(dup_view.duplicates, 2);
        assert_eq!(dup_view.files[0].duplicate_count, 1);
        assert_eq!(dup_view.files[1].duplicate_count, 1);

        // Get file duplicates for kick1
        let kick1_file = dup_view
            .files
            .iter()
            .find(|f| f.name == "kick_orig.wav")
            .unwrap();
        let dups = catalog.get_file_duplicates(&kick1_file.id).unwrap();
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].relative_path, "subfolder/kick_copy.wav");
        assert_eq!(dups[0].size, kick1_file.size);

        // Non-duplicate snare has 0 duplicates
        let all_files = catalog.query(Query::default()).unwrap();
        let snare_file = all_files
            .files
            .iter()
            .find(|f| f.name == "snare.wav")
            .unwrap();
        assert_eq!(snare_file.duplicate_count, 0);
        let snare_dups = catalog.get_file_duplicates(&snare_file.id).unwrap();
        assert_eq!(snare_dups.len(), 0);
    }

    #[test]
    fn duplicate_hashes_follow_changed_missing_and_restored_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("samples");
        fs::create_dir(&root).unwrap();
        let first = root.join("first.wav");
        let second = root.join("second.wav");
        wav(&first);
        fs::copy(&first, &second).unwrap();
        let catalog = Catalog::open(temp.path().join("catalog.sqlite")).unwrap();
        let root_id = catalog.add_root(&root).unwrap();
        let scan = || {
            catalog
                .scan(&root_id, &AtomicBool::new(false), |_| {})
                .unwrap()
        };
        scan();
        assert_eq!(
            catalog.duplicate_summary().unwrap().duplicate_files_count,
            2
        );
        let first_id = catalog
            .query(Query::default())
            .unwrap()
            .files
            .into_iter()
            .find(|file| file.name == "first.wav")
            .unwrap()
            .id;
        catalog
            .annotate(
                &first_id,
                Annotation {
                    favorite: true,
                    ..Default::default()
                },
            )
            .unwrap();

        // Same byte length, different samples: the previous association must go.
        wav_value(&second, 1);
        catalog.index_duplicates().unwrap();
        assert_eq!(
            catalog.duplicate_summary().unwrap().duplicate_files_count,
            0
        );
        scan();
        assert_eq!(
            catalog.duplicate_summary().unwrap().duplicate_files_count,
            0
        );
        assert!(catalog.get_file_duplicates(&first_id).unwrap().is_empty());
        assert!(
            catalog
                .query(Query::default())
                .unwrap()
                .files
                .iter()
                .find(|f| f.id == first_id)
                .unwrap()
                .favorite
        );

        fs::copy(&first, &second).unwrap();
        scan();
        assert_eq!(
            catalog.duplicate_summary().unwrap().duplicate_files_count,
            2
        );

        fs::remove_file(&second).unwrap();
        scan();
        assert_eq!(
            catalog.duplicate_summary().unwrap().duplicate_files_count,
            0
        );
        assert!(catalog.get_file_duplicates(&first_id).unwrap().is_empty());

        fs::copy(&first, &second).unwrap();
        scan();
        assert_eq!(
            catalog.duplicate_summary().unwrap().duplicate_files_count,
            2
        );
    }

    #[test]
    fn backup_includes_committed_wal_and_can_restore_organization() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("sounds");
        fs::create_dir(&root).unwrap();
        let source = root.join("note.wav");
        wav(&source);
        let original_hash = hash_file(&source).unwrap();
        let catalog = Catalog::open(temp.path().join("catalog.sqlite")).unwrap();
        let root_id = catalog.add_root(&root).unwrap();
        catalog
            .scan(&root_id, &AtomicBool::new(false), |_| {})
            .unwrap();
        let id = catalog.query(Query::default()).unwrap().files[0].id.clone();
        catalog
            .annotate(
                &id,
                Annotation {
                    favorite: true,
                    tags: vec!["idea".into()],
                    notes: "mantener".into(),
                    rating: 4,
                    ..Default::default()
                },
            )
            .unwrap();
        let collection = catalog.create_collection("Ideas", None).unwrap();
        catalog
            .add_to_collection(&collection.id, std::slice::from_ref(&id))
            .unwrap();
        let destination = temp.path().join("backups");
        fs::create_dir(&destination).unwrap();
        let backup = catalog.backup_to(&destination).unwrap();
        let copy = Catalog::open(backup.clone()).unwrap();
        let restored = copy.query(Query::default()).unwrap();
        assert_eq!(restored.files.len(), 1);
        assert_eq!(restored.files[0].tags, vec!["idea"]);
        assert_eq!(restored.files[0].notes, "mantener");
        assert_eq!(restored.files[0].rating, 4);
        assert!(restored.files[0].favorite);
        assert_eq!(restored.collections[0].count, 1);

        let other_dir = temp.path().join("fresh-app");
        fs::create_dir(&other_dir).unwrap();
        let active = Catalog::open(other_dir.join("catalog.sqlite")).unwrap();
        active.create_collection("Actual", None).unwrap();
        fs::write(temp.path().join("invalid.sqlite"), "not a database").unwrap();
        assert!(Catalog::stage_restore(&temp.path().join("invalid.sqlite"), &other_dir).is_err());
        assert_eq!(active.list_collections().unwrap()[0].name, "Actual");
        Catalog::stage_restore(&backup, &other_dir).unwrap();
        assert_eq!(active.list_collections().unwrap()[0].name, "Actual");
        let recovery = Catalog::apply_pending_restore(&other_dir).unwrap().unwrap();
        let changed = Catalog::open(other_dir.join("catalog.sqlite")).unwrap();
        assert_eq!(changed.list_collections().unwrap()[0].name, "Ideas");
        assert_eq!(
            changed.query(Query::default()).unwrap().files[0].notes,
            "mantener"
        );
        let previous = Catalog::open(recovery).unwrap();
        assert_eq!(previous.list_collections().unwrap()[0].name, "Actual");
        assert!(!other_dir.join("catalog-restore-pending.sqlite").exists());
        fs::write(other_dir.join("catalog-restore-pending.sqlite"), "corrupt").unwrap();
        assert!(Catalog::apply_pending_restore(&other_dir).is_err());
        assert_eq!(changed.list_collections().unwrap()[0].name, "Ideas");
        assert_eq!(hash_file(&source).unwrap(), original_hash);
    }
}
