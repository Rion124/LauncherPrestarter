use anyhow::Result;
use std::{env, path::PathBuf, process::Command};

pub fn java_executable_file(java_dir: &PathBuf) -> PathBuf {

    #[cfg(target_family = "windows")]
    let java_exe = java_dir.join("bin/javaw.exe");

    #[cfg(target_family = "unix")]
    let java_exe = java_dir.join("bin/java");

    java_exe
}

/// Starts the launcher with the downloaded Java.
///
/// `launcher_jar` is either the jar downloaded from the server (download mode)
/// or this executable itself — a jar appended to the exe stays readable as a zip,
/// which is how upstream ships the launcher.
pub fn relaunch_using_java(java_dir: &PathBuf, launcher_jar: &PathBuf) -> Result<()> {
    let java_exe = java_executable_file(java_dir);

    let args: Vec<String> = env::args().skip(1).collect();

    Command::new(java_exe)
        .arg("-Dlauncher.noJavaCheck=true")
        .arg("-jar")
        .arg(launcher_jar)
        .args(args)
        .spawn()?; // not waiting intentionally

    Ok(())
}

/// The jar handed to Java when it is bundled with this executable.
pub fn self_jar() -> Result<PathBuf> {
    Ok(env::current_exe()?)
}
