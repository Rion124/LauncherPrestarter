fn main() {
    // Read with option_env! at compile time (see src/launcher.rs). Cargo does not
    // track environment variables on its own, so without this a cached build would
    // silently keep the previous launcher URL.
    println!("cargo:rerun-if-env-changed=PRESTARTER_LAUNCHER_URL");
    println!("cargo:rerun-if-env-changed=PRESTARTER_LAUNCHER_SHA256");
    println!("cargo:rerun-if-env-changed=PRESTARTER_STORE_DIR");
    // The icon is baked into the executable's resources by tauri-build. Cargo
    // does not know that, so without this a changed icon keeps the previously
    // generated resource and the exe silently ships the old one.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
