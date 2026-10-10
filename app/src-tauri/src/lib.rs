/// Starts the Lupa window. Exits with code 1 when Tauri cannot start.
pub fn run() {
    if let Err(error) = tauri::Builder::default().run(tauri::generate_context!()) {
        eprintln!("Lupa failed to start: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    const CONFIG: &str = include_str!("../tauri.conf.json");

    /// Sources the production webview may load from. Untrusted MR content is
    /// rendered in this webview, so anything remote must stay out of the CSP.
    const ALLOWED_SOURCES: &[&str] = &["'self'", "'none'", "data:", "ipc:", "http://ipc.localhost"];

    fn production_csp() -> String {
        let config: Value = serde_json::from_str(CONFIG).unwrap();
        config["app"]["security"]["csp"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn production_csp_allows_only_local_sources() {
        let csp = production_csp();
        let directives: Vec<&str> = csp
            .split(';')
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .collect();
        assert!(
            directives.contains(&"default-src 'self'"),
            "missing default-src 'self' in {csp}"
        );
        for directive in directives {
            for source in directive.split_whitespace().skip(1) {
                assert!(
                    ALLOWED_SOURCES.contains(&source),
                    "source {source} in directive `{directive}` is not local"
                );
            }
        }
    }
}
