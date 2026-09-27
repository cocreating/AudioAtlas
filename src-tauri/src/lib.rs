pub mod audio_stream;
pub mod catalog;
mod commands;
pub mod playback;
pub mod waveform;

use std::sync::{atomic::AtomicBool, Arc};
use tauri::Manager;

pub struct AppState {
    pub catalog: catalog::Catalog,
    pub playback: playback::Playback,
    pub waveforms: waveform::Waveforms,
    pub scanning: Arc<AtomicBool>,
    pub cancel_scan: Arc<AtomicBool>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            app.manage(AppState {
                catalog: catalog::Catalog::open(directory.join("catalog.sqlite"))
                    .map_err(|e| std::io::Error::other(e.to_string()))?,
                playback: playback::Playback::start(),
                waveforms: waveform::Waveforms::new(app.path().app_cache_dir()?.join("waveforms")),
                scanning: Arc::new(AtomicBool::new(false)),
                cancel_scan: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library,
            commands::choose_root,
            commands::rescan,
            commands::cancel_scan,
            commands::annotate,
            commands::play,
            commands::transport,
            commands::player_state,
            commands::export_files,
            commands::reveal,
            commands::waveform,
            commands::cancel_waveform,
            commands::create_collection,
            commands::rename_collection,
            commands::delete_collection,
            commands::add_to_collection,
            commands::remove_from_collection,
            commands::save_smart_query,
            commands::delete_smart_query
        ])
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar Audio Atlas");
}
