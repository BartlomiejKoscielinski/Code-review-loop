//! IPC commands exposed to the webview. Each command is also listed in
//! `build.rs`, which generates its `allow-*` permission, and is granted to the
//! main window in `capabilities/default.json`.

use serde::Serialize;

use crate::ipc::IpcError;

/// Basic facts about the running application, shown in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle) -> Result<AppInfo, IpcError> {
    let package = app.package_info();
    Ok(AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
    })
}
