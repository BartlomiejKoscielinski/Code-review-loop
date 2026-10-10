pub mod command_names;
pub mod commands;
pub mod ipc;
pub mod navigation;

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, WebviewWindowBuilder};

/// Starts the Lupa window and blocks until it is closed.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .setup(|app| create_main_window(app.handle()))
        .run(tauri::generate_context!())
}

/// Creates the main window from `tauri.conf.json` (where it has
/// `"create": false`), adding the navigation guard and blocking new windows.
fn create_main_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .ok_or("tauri.conf.json has no window labelled main")?
        .clone();
    WebviewWindowBuilder::from_config(app, &config)?
        .on_navigation(|url| navigation::is_app_url(url, cfg!(dev)))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;

    const CONFIG: &str = include_str!("../tauri.conf.json");

    /// Sources each directive of the production CSP may use. The webview shows
    /// untrusted MR content, so nothing remote and nothing inline is allowed,
    /// and `data:` and IPC sources stay in the one directive that needs them.
    const ALLOWED: &[(&str, &[&str])] = &[
        ("default-src", &["'self'"]),
        ("script-src", &["'self'"]),
        ("style-src", &["'self'"]),
        ("img-src", &["'self'", "data:"]),
        ("font-src", &["'self'"]),
        ("connect-src", &["ipc:", "http://ipc.localhost"]),
        ("object-src", &["'none'"]),
        ("base-uri", &["'self'"]),
        ("form-action", &["'none'"]),
        ("frame-ancestors", &["'none'"]),
    ];

    fn security() -> Value {
        let config: Value = serde_json::from_str(CONFIG).unwrap();
        config["app"]["security"].clone()
    }

    #[test]
    fn production_csp_uses_only_allowed_sources_per_directive() {
        let security = security();
        let csp = security["csp"].as_str().unwrap();
        let directives: Vec<&str> = csp
            .split(';')
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .collect();
        assert!(
            directives.iter().any(|d| d.starts_with("default-src ")),
            "missing default-src in {csp}"
        );
        for directive in directives {
            let mut parts = directive.split_whitespace();
            let name = parts.next().unwrap();
            let allowed = ALLOWED
                .iter()
                .find(|(known, _)| *known == name)
                .map(|(_, sources)| *sources)
                .unwrap_or_else(|| panic!("directive {name} has no allow list in this test"));
            for source in parts {
                assert!(
                    allowed.contains(&source),
                    "source {source} is not allowed in `{directive}`"
                );
            }
        }
    }

    #[test]
    fn asset_csp_and_asset_protocol_stay_disabled() {
        let security = security();
        assert!(
            security
                .get("dangerousDisableAssetCspModification")
                .is_none(),
            "Tauri must keep adding CSP nonces and hashes to assets"
        );
        assert_ne!(security["assetProtocol"]["enable"], Value::Bool(true));
    }

    #[test]
    fn capabilities_grant_exactly_the_app_commands_to_the_main_window() {
        assert!(
            security().get("capabilities").is_none(),
            "inline capabilities in tauri.conf.json bypass the files checked below"
        );
        let expected: BTreeSet<String> = crate::command_names::COMMANDS
            .iter()
            .map(|command| format!("allow-{}", command.replace('_', "-")))
            .collect();
        assert!(
            !expected.is_empty(),
            "an empty command list disables the app ACL"
        );

        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities");
        let mut granted = BTreeSet::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let file = path.display().to_string();
            let capability: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            for key in ["remote", "webviews", "platforms"] {
                assert!(
                    capability.get(key).is_none(),
                    "{file}: `{key}` is not allowed"
                );
            }
            assert_ne!(capability["local"], Value::Bool(false), "{file}");
            assert_eq!(capability["windows"], serde_json::json!(["main"]), "{file}");
            for permission in capability["permissions"].as_array().unwrap() {
                granted.insert(permission.as_str().unwrap().to_owned());
            }
        }
        assert_eq!(granted, expected);
    }

    #[test]
    fn main_window_is_created_in_code_with_the_navigation_guard() {
        let config: Value = serde_json::from_str(CONFIG).unwrap();
        let windows = config["app"]["windows"].as_array().unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0]["label"], "main");
        assert_eq!(windows[0]["create"], Value::Bool(false));
        assert_eq!(security()["freezePrototype"], Value::Bool(true));
    }
}
