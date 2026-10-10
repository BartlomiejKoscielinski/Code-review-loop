//! Error type shared by all IPC commands.
//!
//! Every command returns `Result<T, IpcError>`. The message is shown to the
//! user, so it is built from context only and is scrubbed of anything that
//! looks like a GitLab token before it leaves the core.

use serde::Serialize;

/// Stable error category the UI can branch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Unexpected failure inside the core.
    Internal,
    /// The UI sent arguments the command does not accept.
    InvalidInput,
}

/// Error returned by every IPC command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct IpcError {
    code: ErrorCode,
    message: String,
}

impl IpcError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: redact_tokens(&message.into()),
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Prefixes of GitLab token types (personal, OAuth, deploy, runner, CI job,
/// trigger, feed, incoming mail, agent, SCIM, feature flags client).
const TOKEN_PREFIXES: &[&str] = &[
    "glpat-", "gloas-", "gldt-", "glrt-", "glcbt-", "glptt-", "glft-", "glimt-", "glagent-",
    "glsoat-", "glffct-",
];

const REDACTED: &str = "[REDACTED]";

/// Replaces secrets that commonly leak into error text with `[REDACTED]`:
/// GitLab-token-shaped substrings and the password part of `scheme://user:password@host`.
///
/// This is a safety net for messages built from external output (for example
/// a `git` error that echoes a URL). Code must still never put the token into
/// a message on purpose; P1-14a also removes the exact stored token value.
pub fn redact_tokens(text: &str) -> String {
    redact_url_passwords(&redact_prefixed_tokens(text))
}

fn redact_prefixed_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied_until = 0;
    let mut search_from = 0;
    while let Some(offset) = text[search_from..].find("gl") {
        let start = search_from + offset;
        let prefix = TOKEN_PREFIXES
            .iter()
            .find(|prefix| text[start..].starts_with(*prefix));
        let Some(prefix) = prefix else {
            search_from = start + 2;
            continue;
        };
        let token_start = start + prefix.len();
        let token_len = token_length(&text[token_start..]);
        if token_len == 0 {
            search_from = token_start;
            continue;
        }
        out.push_str(&text[copied_until..start]);
        out.push_str(REDACTED);
        copied_until = token_start + token_len;
        search_from = copied_until;
    }
    out.push_str(&text[copied_until..]);
    out
}

fn token_length(text: &str) -> usize {
    text.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
        .unwrap_or(text.len())
}

/// `https://user:secret@host` becomes `https://user:[REDACTED]@host`.
fn redact_url_passwords(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(scheme_end) = rest.find("://") {
        let authority_start = scheme_end + 3;
        out.push_str(&rest[..authority_start]);
        rest = &rest[authority_start..];
        let authority_len = rest
            .find(|c: char| c == '/' || c.is_whitespace())
            .unwrap_or(rest.len());
        let authority = &rest[..authority_len];
        match (authority.rfind('@'), authority.find(':')) {
            (Some(at), Some(colon)) if colon < at && at > colon + 1 => {
                out.push_str(&authority[..=colon]);
                out.push_str(REDACTED);
                out.push_str(&authority[at..]);
            }
            _ => out.push_str(authority),
        }
        rest = &rest[authority_len..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_personal_access_token_in_url() {
        let message = "fetch failed: https://oauth2:glpat-AbC123_xyz-987@gitlab.local/app.git";
        assert_eq!(
            redact_tokens(message),
            "fetch failed: https://oauth2:[REDACTED]@gitlab.local/app.git"
        );
    }

    #[test]
    fn redacts_every_token_and_keeps_surrounding_text() {
        let message = "a glrt-111 b gldt-222.x c";
        assert_eq!(redact_tokens(message), "a [REDACTED] b [REDACTED] c");
    }

    #[test]
    fn redacts_password_of_url_without_token_prefix() {
        let message =
            "fatal: unable to access 'https://bartek:legacy0123456789@gitlab.local/app.git/'";
        assert_eq!(
            redact_tokens(message),
            "fatal: unable to access 'https://bartek:[REDACTED]@gitlab.local/app.git/'"
        );
    }

    #[test]
    fn keeps_urls_without_password() {
        let message =
            "see https://gitlab.local/app/-/merge_requests/12 and git@gitlab.local:app.git";
        assert_eq!(redact_tokens(message), message);
    }

    #[test]
    fn keeps_bare_prefix_without_token() {
        assert_eq!(redact_tokens("prefix glpat- only"), "prefix glpat- only");
    }

    #[test]
    fn serialized_error_never_contains_the_token() {
        let error = IpcError::new(ErrorCode::Internal, "token glpat-secretvalue123 rejected");
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("secretvalue123"), "{json}");
        assert_eq!(
            json,
            r#"{"code":"internal","message":"token [REDACTED] rejected"}"#
        );
    }
}
