//! Which URLs the main window may navigate to.
//!
//! The window shows untrusted merge request content. Navigation is limited to
//! the app's own pages, so a link in an MR cannot replace Lupa with a page that
//! imitates it (for example to ask for a GitLab token).

use tauri::Url;

/// Vite dev server from `build.devUrl` in `tauri.conf.json`.
const DEV_ORIGIN: (&str, u16) = ("localhost", 1420);

/// True for the app's own pages: `tauri://localhost` (Linux, macOS),
/// `http(s)://tauri.localhost` (Windows) and, in dev builds, the Vite server.
pub fn is_app_url(url: &Url, dev: bool) -> bool {
    let host = url.host_str();
    match url.scheme() {
        "tauri" => host == Some("localhost"),
        "http" | "https" => {
            host == Some("tauri.localhost")
                || (dev && host == Some(DEV_ORIGIN.0) && url.port() == Some(DEV_ORIGIN.1))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str, dev: bool) -> bool {
        is_app_url(&Url::parse(url).unwrap(), dev)
    }

    #[test]
    fn allows_app_pages_on_every_platform() {
        assert!(allowed("tauri://localhost/index.html", false));
        assert!(allowed("http://tauri.localhost/", false));
        assert!(allowed("https://tauri.localhost/assets/app.js", false));
    }

    #[test]
    fn rejects_external_and_lookalike_urls() {
        for url in [
            "https://gitlab.example.com/users/sign_in",
            "http://tauri.localhost.evil.example/",
            "https://evil.example/tauri.localhost",
            "tauri://evil.example/",
            "file:///C:/Users/me/secret.txt",
            "data:text/html,<h1>Lupa</h1>",
            "javascript:alert(1)",
        ] {
            assert!(!allowed(url, false), "{url} must be rejected");
            assert!(!allowed(url, true), "{url} must be rejected in dev");
        }
    }

    #[test]
    fn allows_the_dev_server_only_in_dev_builds() {
        assert!(allowed("http://localhost:1420/", true));
        assert!(!allowed("http://localhost:1420/", false));
        assert!(!allowed("http://localhost:8080/", true));
    }
}
