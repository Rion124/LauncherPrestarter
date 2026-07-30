use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
    time::Duration,
};

use crate::{config::target_dir, download::download_file_auto, download::ProgressCallback};

/// URL of a JSON manifest that tells the prestarter where the launcher lives,
/// baked in at build time:
///
/// ```text
/// PRESTARTER_CONFIG_URL=https://host/prestarter.json yarn tauri build
/// ```
///
/// ```json
/// { "launcherUrl": "https://host/ls/Launcher.jar", "launcherSha256": "" }
/// ```
///
/// Everything that can realistically change later — the jar path, an extra
/// checksum — is edited on the server instead of shipping players a new exe.
/// Several comma-separated URLs may be given; they are tried in order, so a
/// second host can act as a fallback. The last manifest that worked is cached on
/// disk and reused when the server cannot be reached.
pub fn config_url() -> Option<&'static str> {
    non_empty(option_env!("PRESTARTER_CONFIG_URL"))
}

/// Remote manifest, as served by [`config_url`].
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteConfig {
    pub launcher_url: Option<String>,
    pub launcher_sha256: Option<String>,
}

static REMOTE: std::sync::OnceLock<RemoteConfig> = std::sync::OnceLock::new();

fn remote_cache_path() -> Option<PathBuf> {
    Some(target_dir().ok()?.join("prestarter-remote.json"))
}

/// Fetched once per run: the manifest from the server, or the cached copy of the
/// last one that worked, or empty (then the baked-in values are used).
fn remote_config() -> &'static RemoteConfig {
    REMOTE.get_or_init(|| {
        let Some(urls) = config_url() else {
            return RemoteConfig::default();
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .ok();
        if let Some(client) = client {
            for url in urls.split(',').map(str::trim).filter(|u| !u.is_empty()) {
                let fetched = client
                    .get(url)
                    .send()
                    .ok()
                    .and_then(|r| r.error_for_status().ok())
                    .and_then(|r| r.text().ok())
                    .and_then(|body| serde_json::from_str::<RemoteConfig>(&body).ok());
                if let Some(config) = fetched {
                    if let Some(path) = remote_cache_path() {
                        if let Some(parent) = path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(json) = serde_json::to_string_pretty(&config) {
                            let _ = fs::write(&path, json);
                        }
                    }
                    return config;
                }
            }
        }
        // Offline: fall back to the last manifest that was fetched successfully.
        remote_cache_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<RemoteConfig>(&s).ok())
            .unwrap_or_default()
    })
}

/// Where to download the launcher jar from: the manifest wins, otherwise the URL
/// baked in at build time (`PRESTARTER_LAUNCHER_URL`).
///
/// With neither set the prestarter keeps the upstream behaviour: the jar is
/// expected to be appended to this executable and Java is pointed at the exe
/// itself. Either one switches to download mode, which keeps the exe small
/// (~5 MB instead of ~23 MB) and lets the launcher update on its own.
pub fn launcher_url() -> Option<&'static str> {
    if let Some(url) = remote_config().launcher_url.as_deref() {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    non_empty(option_env!("PRESTARTER_LAUNCHER_URL"))
}

/// Optional SHA-256 of the launcher jar, baked in at build time. When set, a
/// downloaded jar MUST match it or it is rejected — the strongest option, but
/// the exe has to be rebuilt for every launcher update. When unset, the
/// prestarter asks the server for `<url>.sha256` instead (see
/// [`remote_expected_hash`]), which allows launcher updates without touching the
/// exe but only authenticates the download if the URL is HTTPS.
pub fn pinned_sha256() -> Option<&'static str> {
    non_empty(option_env!("PRESTARTER_LAUNCHER_SHA256"))
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    match value {
        Some(v) if !v.trim().is_empty() => Some(v.trim()),
        _ => None,
    }
}

/// True when the launcher jar is downloaded instead of being appended to the exe.
pub fn is_download_mode() -> bool {
    launcher_url().is_some()
}

/// Where the downloaded launcher jar is cached.
pub fn launcher_jar_path() -> Result<PathBuf> {
    Ok(target_dir()?.join("Launcher.jar"))
}

/// The jar checksum published next to it by the server (`Launcher.jar.sha256`).
/// Used to notice that the launcher was updated. Returns `None` when the server
/// does not serve it or is unreachable, in which case a cached jar is kept.
fn remote_expected_hash() -> Option<String> {
    let url = format!("{}.sha256", launcher_url()?);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .ok()?;
    let body = client.get(&url).send().ok()?.error_for_status().ok()?.text().ok()?;
    let hash = body.split_whitespace().next()?.to_lowercase();
    if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hash)
    } else {
        None
    }
}

/// The checksum a downloaded jar has to match, if any is available at all:
/// the baked-in pin wins, then the manifest, then the file served next to the jar.
fn expected_hash() -> Option<String> {
    if let Some(pinned) = pinned_sha256() {
        return Some(pinned.to_lowercase());
    }
    if let Some(from_manifest) = remote_config().launcher_sha256.as_deref() {
        let hash = from_manifest.trim().to_lowercase();
        if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(hash);
        }
    }
    remote_expected_hash()
}

/// A cached jar that can be launched straight away, without showing any UI.
///
/// A pinned checksum must always match. Otherwise the server is asked for the
/// current checksum: if it differs the jar is stale (the caller downloads it),
/// and if the server cannot be reached the cached jar is used as-is so that the
/// launcher still starts offline.
pub fn cached_jar_ready() -> Option<PathBuf> {
    let path = launcher_jar_path().ok()?;
    if !path.exists() {
        return None;
    }
    match expected_hash() {
        Some(expected) => match sha256_file(&path) {
            Ok(actual) if actual.eq_ignore_ascii_case(&expected) => Some(path),
            _ => None,
        },
        // Nothing to compare against (offline, or no checksum published) — start
        // with the jar we already have rather than refusing to launch.
        None => Some(path),
    }
}

/// Downloads the launcher jar unless an up to date copy is already cached, and
/// returns the path to launch. The download goes to a temporary file first so an
/// interrupted or corrupted transfer can never replace a working launcher.
pub fn ensure_launcher_jar(progress: &ProgressCallback) -> Result<PathBuf> {
    let url = launcher_url().ok_or_else(|| anyhow!("Launcher URL is not configured"))?;
    let path = launcher_jar_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let expected = expected_hash();

    if path.exists() {
        match &expected {
            Some(hash) => {
                if sha256_file(&path)?.eq_ignore_ascii_case(hash) {
                    return Ok(path);
                }
            }
            // Nothing to compare against (offline, or no checksum served) —
            // keep what we have rather than re-downloading on every start.
            None => return Ok(path),
        }
    }

    let temp = path.with_extension("jar.part");
    download_file_auto(url, &temp, progress)?;

    if let Some(hash) = &expected {
        let actual = sha256_file(&temp)?;
        if !actual.eq_ignore_ascii_case(hash) {
            let _ = fs::remove_file(&temp);
            return Err(anyhow!(
                "Launcher checksum mismatch: expected {}, got {}",
                hash,
                actual
            ));
        }
    }

    // Windows will not rename onto an existing file.
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&temp, &path)?;
    Ok(path)
}

fn sha256_file(path: &PathBuf) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}
