use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
    time::Duration,
};

use crate::{config::target_dir, download::download_file_auto, download::ProgressCallback};

/// URL of the launcher jar, baked in at build time:
///
/// ```text
/// PRESTARTER_LAUNCHER_URL=https://host/Launcher.jar yarn tauri build
/// ```
///
/// When it is not set the prestarter keeps the upstream behaviour: the launcher
/// jar is expected to be appended to this executable and Java is pointed at the
/// exe itself. Setting it switches to download mode, which keeps the exe small
/// (~5 MB instead of ~23 MB) and lets the launcher update without shipping a new
/// exe to players.
pub fn launcher_url() -> Option<&'static str> {
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

/// The checksum a downloaded jar has to match, if any is available at all.
fn expected_hash() -> Option<String> {
    match pinned_sha256() {
        Some(pinned) => Some(pinned.to_lowercase()),
        None => remote_expected_hash(),
    }
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
    if let Some(pinned) = pinned_sha256() {
        return match sha256_file(&path) {
            Ok(actual) if actual.eq_ignore_ascii_case(pinned) => Some(path),
            _ => None,
        };
    }
    match remote_expected_hash() {
        Some(expected) => match sha256_file(&path) {
            Ok(actual) if actual.eq_ignore_ascii_case(&expected) => Some(path),
            _ => None,
        },
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
