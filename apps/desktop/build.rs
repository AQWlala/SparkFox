#[path = "../build-support/ui_build_manifest.rs"]
mod ui_build_manifest;

fn main() {
    ui_build_manifest::embed_frontend_build_id("sparkfox-desktop");
    tauri_build::build()
}
