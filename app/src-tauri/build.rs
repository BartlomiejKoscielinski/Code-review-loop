// Commands listed here get generated `allow-<command>` permissions. The webview
// may call a command only if a capability grants that permission.
const COMMANDS: &[&str] = &["app_info"];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    if let Err(error) = tauri_build::try_build(attributes) {
        // Build scripts report failures by panicking; there is no caller to return to.
        panic!("tauri build step failed: {error:#}");
    }
}
