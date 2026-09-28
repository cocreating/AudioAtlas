use crate::{
    catalog::{
        Annotation, Collection, DuplicateLocation, DuplicateSummary, ExportResult, Library, Query,
        SmartQuery,
    },
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
pub async fn backup_catalog(app: AppHandle) -> Result<Option<String>, String> {
    let dialog_app = app.clone();
    let destination = blocking(move || {
        Ok(dialog_app
            .dialog()
            .file()
            .set_title("Elegir carpeta para respaldar el catálogo")
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
            .backup_to(&path)
            .map(|file| Some(file.to_string_lossy().into_owned()))
            .map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub async fn prepare_catalog_restore(app: AppHandle) -> Result<bool, String> {
    if app.state::<AppState>().scanning.load(Ordering::SeqCst) {
        return Err("Espera a que termine el escaneo".into());
    }
    let dialog_app = app.clone();
    let source = blocking(move || {
        Ok(dialog_app
            .dialog()
            .file()
            .set_title("Elegir respaldo del catálogo")
            .add_filter("Catálogo SQLite", &["sqlite"])
            .blocking_pick_file())
    })
    .await?;
    let Some(source) = source else {
        return Ok(false);
    };
    let path = source.into_path().map_err(|e| e.to_string())?;
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?;
    blocking(move || {
        crate::catalog::Catalog::stage_restore(&path, &directory)
            .map(|()| true)
            .map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub fn startup_issue(app: AppHandle) -> Option<String> {
    app.state::<AppState>().startup_issue.clone()
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

#[tauri::command]
pub async fn waveform(app: AppHandle, id: String) -> Result<crate::waveform::Waveform, String> {
    let ticket = app.state::<AppState>().waveforms.begin();
    blocking(move || {
        let state = app.state::<AppState>();
        let path = state.catalog.resolve(&id).map_err(|e| e.to_string())?;
        state.waveforms.get(&path, ticket)
    })
    .await
}
#[tauri::command]
pub fn cancel_waveform(app: AppHandle) {
    app.state::<AppState>().waveforms.cancel();
}

#[tauri::command]
pub async fn create_collection(
    app: AppHandle,
    name: String,
    color: Option<String>,
) -> Result<Collection, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .create_collection(&name, color.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn rename_collection(
    app: AppHandle,
    id: String,
    name: String,
    color: Option<String>,
) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .rename_collection(&id, &name, color.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_collection(app: AppHandle, id: String) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || catalog.delete_collection(&id).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn add_to_collection(
    app: AppHandle,
    collection_id: String,
    file_ids: Vec<String>,
) -> Result<usize, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .add_to_collection(&collection_id, &file_ids)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn remove_from_collection(
    app: AppHandle,
    collection_id: String,
    file_ids: Vec<String>,
) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .remove_from_collection(&collection_id, &file_ids)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn save_smart_query(
    app: AppHandle,
    name: String,
    filter_json: String,
) -> Result<SmartQuery, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .save_smart_query(&name, &filter_json)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_smart_query(app: AppHandle, id: String) -> Result<(), String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || catalog.delete_smart_query(&id).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn scan_duplicates(app: AppHandle) -> Result<DuplicateSummary, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog.index_duplicates().map_err(|e| e.to_string())?;
        catalog.duplicate_summary().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn get_file_duplicates(
    app: AppHandle,
    file_id: String,
) -> Result<Vec<DuplicateLocation>, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || {
        catalog
            .get_file_duplicates(&file_id)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn duplicate_summary(app: AppHandle) -> Result<DuplicateSummary, String> {
    let catalog = app.state::<AppState>().catalog.clone();
    blocking(move || catalog.duplicate_summary().map_err(|e| e.to_string())).await
}
