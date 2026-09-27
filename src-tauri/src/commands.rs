use crate::{
    catalog::{Annotation, ExportResult, Library, Query},
    playback::{Control, PlayerState},
    AppState,
};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}
fn scan(app: AppHandle, root_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Ya hay un escaneo en curso".into());
    }
    state.cancel_scan.store(false, Ordering::SeqCst);
    let catalog = state.catalog.clone();
    let cancel = state.cancel_scan.clone();
    let scanning = state.scanning.clone();
    std::thread::spawn(move || {
        let result = catalog.scan(&root_id, &cancel, |p| {
            let _ = app.emit("scan-progress", p);
        });
        scanning.store(false, Ordering::SeqCst);
        if let Err(error) = result {
            let _ = app.emit("scan-error", error.to_string());
        }
        let _ = app.emit("scan-finished", ());
    });
    Ok(())
}
#[tauri::command]
pub async fn library(app: AppHandle, query: Query) -> Result<Library, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    let scanning = app.state::<AppState>().scanning.load(Ordering::SeqCst);
    blocking(move || {
        let mut result = catalog.query(query).map_err(|e| e.to_string())?;
        result.scanning = scanning;
        Ok(result)
    })
    .await
}
#[tauri::command]
pub async fn choose_root(app: AppHandle) -> Result<Option<String>, String> {
    let dialog_app = app.clone();
    let root = blocking(move || {
        Ok(dialog_app
            .dialog()
            .file()
            .set_title("Añadir una fuente de audio")
            .blocking_pick_folder())
    })
    .await?;
    let Some(root) = root else { return Ok(None) };
    let path = root.into_path().map_err(|e| e.to_string())?;
    let catalog = app.state::<AppState>().catalog.clone();
    let id = blocking(move || catalog.add_root(&path).map_err(|e| e.to_string())).await?;
    scan(app, id.clone())?;
    Ok(Some(id))
}
#[tauri::command]
pub fn rescan(app: AppHandle, root_id: String) -> Result<(), String> {
    scan(app, root_id)
}
#[tauri::command]
pub fn cancel_scan(app: AppHandle) {
    app.state::<AppState>()
        .cancel_scan
        .store(true, Ordering::SeqCst);
}
#[tauri::command]
pub async fn annotate(app: AppHandle, id: String, annotation: Annotation) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || catalog.annotate(&id, annotation).map_err(|e| e.to_string())).await
}
#[tauri::command]
pub async fn play(app: AppHandle, id: String) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    let player = app.state::<AppState>().playback.clone();
    blocking(move || {
        let path = catalog.resolve(&id).map_err(|e| e.to_string())?;
        player.play(id, path)
    })
    .await
}
#[tauri::command]
pub async fn transport(app: AppHandle, control: Control) -> Result<(), String> {
    let player = app.state::<AppState>().playback.clone();
    blocking(move || player.control(control)).await
}
#[tauri::command]
pub async fn player_state(app: AppHandle) -> Result<PlayerState, String> {
    let player = app.state::<AppState>().playback.clone();
    blocking(move || player.state()).await
}
#[tauri::command]
pub async fn export_files(
    app: AppHandle,
    ids: Vec<String>,
) -> Result<Option<ExportResult>, String> {
    let dialog_app = app.clone();
    let destination = blocking(move || {
        Ok(dialog_app
            .dialog()
            .file()
            .set_title("Elegir destino para las copias")
            .blocking_pick_folder())
    })
    .await?;
    let Some(destination) = destination else {
        return Ok(None);
    };
    let path = destination.into_path().map_err(|e| e.to_string())?;
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .export(&ids, &path)
            .map(Some)
            .map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub async fn reveal(app: AppHandle, id: String) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        let path = catalog.resolve(&id).map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("/usr/bin/open")
                .arg("-R")
                .arg(path)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err("Finder no pudo mostrar el archivo".into());
            }
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            Err("Revelar archivos sólo está implementado en macOS".into())
        }
    })
    .await
}
