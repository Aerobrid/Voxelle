//! Text-insertion boundary (P0 §0.3, security.md §3). Order: native → clipboard paste → manual
//! copy. Always strips control characters and never auto-submits (no trailing Enter). Platform
//! adapters land in the insertion slice; the stub never claims a false "inserted".

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InsertOutcome {
    Inserted,
    CopiedOnly,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertResult {
    pub outcome: InsertOutcome,
    pub message: Option<String>,
}

#[derive(Debug, Error)]
pub enum InsertError {
    #[error("insertion target rejected input (secure field / elevated app / remote desktop)")]
    TargetRejected,
    #[error("insertion not available: {0}")]
    Unavailable(String),
}

/// Remove control characters (keep normal whitespace) and strip any trailing newline so insertion
/// can never submit a form/line. Used by every inserter before touching the OS.
pub fn sanitize_for_insertion(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\t' || *c == '\n' || *c == '\r')
        .collect();
    cleaned.trim_end_matches(['\n', '\r']).to_string()
}

pub trait TextInserter: Send {
    /// Attempt to place `text` at the cursor. Returns the honest outcome.
    fn insert(&mut self, text: &str) -> InsertResult;
    fn describe(&self) -> &'static str;
}

/// Placeholder until platform adapters are wired. Never reports "inserted".
pub struct StubInserter;

impl TextInserter for StubInserter {
    fn insert(&mut self, _text: &str) -> InsertResult {
        InsertResult {
            outcome: InsertOutcome::Failed,
            message: Some("text inserter not yet wired (P0 insertion slice)".into()),
        }
    }
    fn describe(&self) -> &'static str {
        "stub (no platform adapter)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_control_chars_but_keeps_tabs_and_internal_newlines() {
        let input = "he\u{0007}llo\tworld\nsecond\u{0000}line";
        assert_eq!(sanitize_for_insertion(input), "hello\tworld\nsecondline");
    }

    #[test]
    fn never_leaves_trailing_newline() {
        assert_eq!(sanitize_for_insertion("done\n"), "done");
        assert_eq!(sanitize_for_insertion("done\r\n"), "done");
        assert_eq!(sanitize_for_insertion("done\n\n"), "done");
    }

    #[test]
    fn keeps_plain_text_unchanged() {
        assert_eq!(sanitize_for_insertion("just text"), "just text");
    }
}
