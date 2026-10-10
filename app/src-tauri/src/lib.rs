pub mod commands;
pub mod ipc;

/// Starts the Lupa window and blocks until it is closed.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
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
    fn capabilities_grant_only_app_commands_to_the_main_window() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            let capability: Value = serde_json::from_str(&text).unwrap();
            let file = path.display();
            assert!(
                capability.get("remote").is_none(),
                "{file} exposes IPC to remote URLs"
            );
            assert_eq!(capability["windows"], serde_json::json!(["main"]), "{file}");
            for permission in capability["permissions"].as_array().unwrap() {
                let name = permission.as_str().unwrap();
                assert!(
                    name.starts_with("allow-") && !name.contains(':'),
                    "{file}: permission {name} is not an app command permission"
                );
            }
            checked += 1;
        }
        assert!(checked > 0, "no capability files in {dir}");
    }
}
