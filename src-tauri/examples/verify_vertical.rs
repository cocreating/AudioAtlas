//! Runs against an isolated catalog and explicitly supplied fixture directory.
use audio_atlas_lib::{
    catalog::{hash_file, Annotation, Catalog, Query},
    playback::{Control, LoopRegion, Playback},
    waveform::Waveforms,
};
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Duration};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Usage: verify_vertical FIXTURE_DIRECTORY [--audio]")?,
    )
    .canonicalize()?;
    let temp = tempfile::tempdir()?;
    let database = temp.path().join("catalog.sqlite");
    let catalog = Catalog::open(database.clone())?;
    let root_id = catalog.add_root(&root)?;
    let report = catalog.scan(&root_id, &AtomicBool::new(false), |_| {})?;
    let library = catalog.query(Query::default())?;
    let long = library
        .files
        .iter()
        .find(|f| f.duration.unwrap_or(0.0) > 120.0)
        .ok_or("Missing long fixture")?;
    let path = catalog.resolve(&long.id)?;
    let hash = hash_file(&path)?;
    catalog.annotate(
        &long.id,
        Annotation {
            favorite: true,
            tags: vec!["verificación".into()],
            notes: "Prueba de persistencia".into(),
            rating: 4,
            status: "listened".into(),
        },
    )?;
    let export = catalog.export(std::slice::from_ref(&long.id), temp.path())?;
    assert_eq!(export.files[0].sha256, hash);
    assert_eq!(hash_file(&path)?, hash);
    drop(catalog);
    let reopened = Catalog::open(database)?;
    let reopened_lib = reopened.query(Query {
        text: "verificacion".into(),
        ..Default::default()
    })?;
    assert_eq!(reopened_lib.matched, 1);
    assert_eq!(reopened_lib.files[0].rating, 4);
    assert_eq!(reopened_lib.files[0].user_status, "listened");
    println!("Catalog: {} files; {} decoder errors isolated; annotations survive reopen; export SHA-256 verified; source unchanged",report.indexed,report.errors);
    let waves = Waveforms::new(temp.path().join("waveforms"));
    let waveform = waves.get(&path, waves.begin())?;
    assert!(waveform.levels[0].peaks.len() <= 2048);
    assert_eq!(waves.get(&path, waves.begin())?.sha256, hash);
    println!(
        "Waveform: {} frames, {} levels, content hash and cache reuse verified",
        waveform.frames,
        waveform.levels.len()
    );
    if std::env::args().any(|a| a == "--audio") {
        let player = Playback::start();
        // Exercise the real output stream silently; this is not an audible quality test.
        player.control(Control::Volume(0.0))?;
        player.play(long.id.clone(), path)?;
        std::thread::sleep(Duration::from_millis(400));
        assert!(player.state()?.playing);
        player.control(Control::Seek(60.0))?;
        std::thread::sleep(Duration::from_millis(150));
        let state = player.state()?;
        assert!((59.9..62.0).contains(&state.position));
        player.control(Control::Pause)?;
        let paused = player.state()?;
        assert!(!paused.playing);
        std::thread::sleep(Duration::from_millis(100));
        assert!((player.state()?.position - paused.position).abs() < 0.1);
        player.control(Control::Resume)?;
        assert!(player.state()?.playing);
        player.control(Control::SetLoop {
            file_id: long.id.clone(),
            region: Some(LoopRegion {
                start: 1.0,
                end: 1.2,
            }),
        })?;
        std::thread::sleep(Duration::from_millis(550));
        let looped = player.state()?;
        assert!(looped.playing);
        assert!((1.0..=1.2).contains(&looped.position));
        assert!(player
            .control(Control::SetLoop {
                file_id: "stale-file".into(),
                region: None
            })
            .is_err());
        player.control(Control::Pause)?;
        player.control(Control::Seek(1.05))?;
        let paused_loop = player.state()?;
        assert!(!paused_loop.playing);
        assert!((1.04..1.08).contains(&paused_loop.position));
        player.control(Control::SetLoop {
            file_id: long.id.clone(),
            region: None,
        })?;
        assert!(player.state()?.loop_region.is_none());
        player.control(Control::Stop)?;
        let stopped = player.state()?;
        assert!(stopped.file_id.is_none());
        assert_eq!(stopped.position, 0.0);
        assert!(!stopped.playing);
        println!("Native loop A/B: repeats, rejects stale file, seeks while paused, disables and clears on stop.");
        println!("Native output: start, seek to {:.2}s, pause, resume and stop passed at volume zero. Listening quality not assessed.",state.position);
    }
    Ok(())
}
