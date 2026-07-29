// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/


mod config;
mod download;
mod extract;
mod launcher;
mod runner;

use download::download_file;
use extract::extract_zip;
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::{atomic::AtomicU64, Arc},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Emitter;
use tauri::Manager;

use crate::{
    config::{is_java_outdated, load_version_info, target_dir}, download::fetch_latest_release, extract::extract_tar_gz, runner::relaunch_using_java
};

#[tauri::command]
fn start_download(app_handle: tauri::AppHandle) -> Result<(), String> {
    // We'll spawn a background thread
    std::thread::spawn(move || {
        let arc_handle = Arc::new(app_handle);
        let handle = arc_handle.clone();
        let last_update_time = AtomicU64::new(0);
        let emit_progress = move |downloaded: u64, total: u64| {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            // Prevent event spam (300ms cooldown)
            if current_time > last_update_time.load(std::sync::atomic::Ordering::Relaxed) + 300 {
                let _ = handle.emit("download-progress", ProgressEvent { downloaded, total });
            }
        };

        let handle = arc_handle.clone();
        let emit_extract = move |processed: u64, total: u64| {
            let _ = handle.emit("extract-progress", ExtractEvent { processed, total });
        };

        // Java may already be installed from an earlier run — only download it
        // when it is missing or outdated.
        let jdk_dir = match check_java_ready() {
            Some(dir) => dir,
            None => {
                let handle = arc_handle.clone();
                let release = match fetch_latest_release() {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = handle.emit("error", e.to_string());
                        return;
                    }
                };
                let handle = arc_handle.clone();
                let appdata_dir = match target_dir() {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = handle.emit("error", e.to_string());
                        return;
                    }
                };
                let jdk_dir = appdata_dir.join(format!("JRE-{}", release.featureVersion));
                let zip_path = appdata_dir.join(&release.filename);

                let handle = arc_handle.clone();
                match fs::create_dir_all(&appdata_dir) {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = handle.emit("error", e.to_string());
                        return;
                    }
                }

                let handle = arc_handle.clone();
                match config::save_version_info(&release.version, release.featureVersion) {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = handle.emit("error", e.to_string());
                        return;
                    }
                }

                let handle = arc_handle.clone();
                if let Err(e) = download_file(
                    &release.downloadUrl,
                    &zip_path,
                    release.size,
                    &emit_progress,
                ) {
                    let _ = handle.emit("error", e.to_string());
                    return;
                }

                let handle = arc_handle.clone();
                if release.packageType == "tar.gz" {
                    if let Err(e) = extract_tar_gz(&zip_path, &jdk_dir, &emit_extract, true) {
                        let _ = handle.emit("error", e.to_string());
                        return;
                    }
                } else {
                    if let Err(e) = extract_zip(&zip_path, &jdk_dir, &emit_extract, true) {
                        let _ = handle.emit("error", e.to_string());
                        return;
                    }
                }
                
                // Remove zip
                if let Err(e) = fs::remove_file(&zip_path) {
                    let _ = handle.emit("error", e.to_string());
                    return;
                }

                // Save extracted mark
                {
                    let handle = arc_handle.clone();
                    match File::create(&jdk_dir.join("success-extracted-mark")) {
                        Ok(_) => {}
                        Err(e) => {
                            let _ = handle.emit("error", e.to_string());
                            return;
                        }
                    }
                }

                jdk_dir
            }
        };

        // Launcher jar: downloaded from the server when a URL was baked in at
        // build time, otherwise the copy appended to this executable.
        let handle = arc_handle.clone();
        let launcher_jar = if launcher::is_download_mode() {
            match launcher::ensure_launcher_jar(&emit_progress) {
                Ok(path) => path,
                Err(e) => {
                    let _ = handle.emit("error", e.to_string());
                    return;
                }
            }
        } else {
            match runner::self_jar() {
                Ok(path) => path,
                Err(e) => {
                    let _ = handle.emit("error", e.to_string());
                    return;
                }
            }
        };

        let handle = arc_handle.clone();
        let _ = handle.emit("running", ());

        let handle = arc_handle.clone();
        match relaunch_using_java(&jdk_dir, &launcher_jar) {
            Ok(e) => e,
            Err(e) => {
                let _ = handle.emit("error", e.to_string());
                return;
            }
        }

        let handle = arc_handle.clone();
        let _ = handle.emit("done", ());
    });

    Ok(())
}

#[tauri::command]
fn close_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle.exit(0);
    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct ProgressEvent {
    downloaded: u64,
    total: u64,
}

#[derive(Clone, serde::Serialize)]
struct ExtractEvent {
    processed: u64,
    total: u64,
}



/// The launcher jar to start without showing the window: an up to date cached
/// download, or this executable when the jar is appended to it. `None` means the
/// jar has to be (re)downloaded, so the UI is shown to report progress.
fn ready_launcher_jar() -> Option<PathBuf> {
    if launcher::is_download_mode() {
        launcher::cached_jar_ready()
    } else {
        runner::self_jar().ok()
    }
}

fn check_java_ready() -> Option<PathBuf> {
    let config = load_version_info().ok()??;
    let java_dir = target_dir().ok()?.join(format!("JRE-{}", config.java_feature_version));
    if is_java_outdated(&config) {
        return None;
    }
    if !java_dir.join("success-extracted-mark").exists() {
        return None;
    }
    Some(java_dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    {
        #[cfg(not(dev))]
        // Everything already in place (Java installed and, in download mode, an
        // up to date launcher jar cached): start straight away without any UI.
        if let Some(java_path) = check_java_ready() {
            if let Some(launcher_jar) = ready_launcher_jar() {
                match relaunch_using_java(&java_path, &launcher_jar) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("{}", e.to_string());
                    },
                }
                return;
            }
        }
    }

    #[cfg(target_family = "unix")]
    {
        std::env::set_var("__GL_THREADED_OPTIMIZATIONS", "0");
        std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_download, close_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
