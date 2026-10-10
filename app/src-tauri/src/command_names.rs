/// Every IPC command the webview may call. `build.rs` includes this file to
/// generate one `allow-<command>` permission per entry; a test checks that the
/// capability grants exactly these. Keep in sync with `generate_handler!` in
/// `lib.rs`: a command missing here is rejected by the ACL at runtime.
pub const COMMANDS: &[&str] = &["app_info"];
