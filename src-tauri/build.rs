fn main() {
    // Read with option_env! at compile time (see src/launcher.rs). Cargo does not
    // track environment variables on its own, so without this a cached build would
    // silently keep the previous launcher URL.
    println!("cargo:rerun-if-env-changed=PRESTARTER_LAUNCHER_URL");
    println!("cargo:rerun-if-env-changed=PRESTARTER_LAUNCHER_SHA256");
    println!("cargo:rerun-if-env-changed=PRESTARTER_STORE_DIR");
    tauri_build::build()
}
