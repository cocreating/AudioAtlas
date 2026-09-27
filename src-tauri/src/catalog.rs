use rusqlite::{params, Connection, OptionalExtension};
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
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub text: String,
    pub root_id: Option<String>,
    pub favorites: bool,
    pub format: Option<String>,
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
    pub total: i64,
    pub favorites: i64,
    pub matched: i64,
    pub next: Option<Cursor>,
    pub scanning: bool,
}
#[derive(Deserialize)]
pub struct Annotation {
    pub favorite: bool,
    pub tags: Vec<String>,
    pub notes: String,
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
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 1 {
            return Err("El catálogo pertenece a una versión más reciente".into());
        }
        if version == 0 {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE;\n{}\nCOMMIT;",
                include_str!("../migrations/001_catalog.sql")
            ))?;
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
            for (id, path) in known {
                if !root.join(path).exists() {
                    conn.execute("UPDATE files SET status='missing' WHERE id=?1", [id])?;
                }
            }
            conn.execute(
                "UPDATE roots SET last_scan=CURRENT_TIMESTAMP WHERE id=?1",
                [root_id],
            )?;
        }
        report.done = true;
        progress(report.clone());
        Ok(report)
    }
    pub fn query(&self, query: Query) -> Result<Library> {
        let conn = self.connect()?;
        let roots = conn.prepare("SELECT r.id,r.name,r.path,COUNT(f.id) FROM roots r LEFT JOIN files f ON f.root_id=r.id GROUP BY r.id ORDER BY r.name")?.query_map([], |r| { let path: String = r.get(2)?; Ok(Root {id:r.get(0)?,name:r.get(1)?,online:Path::new(&path).is_dir(),path,count:r.get(3)?}) })?.collect::<std::result::Result<Vec<_>,_>>()?;
        let total = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let favorites = conn.query_row(
            "SELECT COUNT(*) FROM annotations WHERE favorite=1",
            [],
            |r| r.get(0),
        )?;
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
        let base = "FROM files f JOIN annotations a ON a.file_id=f.id WHERE (?1 IS NULL OR f.id IN (SELECT file_id FROM search WHERE search MATCH ?1)) AND (?2 IS NULL OR f.root_id=?2) AND (?3=0 OR a.favorite=1) AND (?4 IS NULL OR f.format=?4)";
        let matched = conn.query_row(
            &format!("SELECT COUNT(*) {base}"),
            params![search, query.root_id, query.favorites, query.format],
            |r| r.get(0),
        )?;
        let sql = format!("SELECT f.id,f.root_id,f.name,f.relative_path,f.size,f.format,f.codec,f.duration,f.sample_rate,f.channels,f.bit_depth,f.status,f.error,a.favorite,a.tags,a.notes {base} AND (?5 IS NULL OR (f.name,f.id)>(?5,?6)) ORDER BY f.name,f.id LIMIT 101");
        let mut files = conn
            .prepare(&sql)?
            .query_map(
                params![
                    search,
                    query.root_id,
                    query.favorites,
                    query.format,
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
            total,
            favorites,
            matched,
            next,
            scanning: false,
        })
    }
    pub fn annotate(&self, id: &str, mut annotation: Annotation) -> Result<()> {
        if annotation.notes.len() > 20000 || annotation.tags.len() > 100 {
            return Err("Las anotaciones superan el tamaño permitido".into());
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
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        let count = tx.execute(
            "UPDATE annotations SET favorite=?1,tags=?2,notes=?3 WHERE file_id=?4",
            params![
                annotation.favorite,
                serde_json::to_string(&annotation.tags)?,
                annotation.notes,
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
            w.write_sample(0i16).unwrap();
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
}
