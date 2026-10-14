//! Text-insertion boundary (P0 §0.3, security.md §3). Order: native typing → clipboard paste →
//! copy-only. Always strips control characters and never auto-submits (no trailing Enter).
//! Strategy selection is pure (`choose_strategy`) so the rules are unit-tested on every platform;
//! the OS adapters only execute the chosen strategy and report what actually happened.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::WindowsInserter;

/// Put text on the system clipboard (used by the UI's Copy buttons). Sanitized like insertion.
pub fn copy_text(text: &str) -> Result<(), String> {
    let text = sanitize_for_insertion(text);
    #[cfg(windows)]
    return self::windows::copy_to_clipboard(&text);
    #[cfg(not(windows))]
    return Err(format!("clipboard copy is not implemented on this platform yet ({} chars)", text.len()));
}

use serde::Serialize;

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
    /// Which adapter produced the outcome (e.g. "typed", "clipboard-paste", "clipboard-copy").
    pub method: String,
    pub message: Option<String>,
}

impl InsertResult {
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            outcome: InsertOutcome::Failed,
            method: "none".into(),
            message: Some(message.into()),
        }
    }
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

/// For platforms without an adapter yet: never claims "inserted".
pub struct UnsupportedInserter;

impl TextInserter for UnsupportedInserter {
    fn insert(&mut self, _text: &str) -> InsertResult {
        InsertResult::failed("Text insertion is not available on this platform yet.")
    }
    fn describe(&self) -> &'static str {
        "unsupported platform"
    }
}

/// What we know about the focused window before inserting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetKind {
    /// No foreground window (e.g. desktop / lock screen).
    None,
    /// Voxelle's own window has focus.
    OwnWindow,
    Normal,
    /// Console hosts / terminal emulators: a newline would run a command.
    Terminal,
    /// Remote desktop client: synthetic Unicode keystrokes are unreliable, clipboard is redirected.
    RemoteDesktop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub kind: TargetKind,
    /// The target runs elevated and we do not (Windows UIPI blocks our input).
    pub elevated_mismatch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strategy {
    /// Synthesize Unicode keystrokes; the clipboard is untouched.
    Type,
    /// Put text on the clipboard, send the paste shortcut, then restore the user's clipboard.
    Paste,
    /// Leave text on the clipboard for the user to paste; report why.
    CopyOnly(String),
}

/// Longest text typed key-by-key; longer text is pasted (faster, no dropped keys).
pub const MAX_TYPED_CHARS: usize = 800;

pub fn choose_strategy(text: &str, target: &Target) -> Strategy {
    let multi_line = text.contains('\n');
    match target.kind {
        TargetKind::None => {
            return Strategy::CopyOnly("No window had focus, so the text was copied instead.".into())
        }
        TargetKind::OwnWindow => {
            return Strategy::CopyOnly(
                "Voxelle itself had focus, so the text was copied. Focus the target app before dictating.".into(),
            )
        }
        _ => {}
    }
    if target.elevated_mismatch {
        return Strategy::CopyOnly(
            "The focused app runs as administrator and Windows blocks input into it from Voxelle. The text was copied; paste it with Ctrl+V.".into(),
        );
    }
    if target.kind == TargetKind::Terminal && multi_line {
        return Strategy::CopyOnly(
            "Multi-line text is not inserted into terminals (a line break could run a command). The text was copied.".into(),
        );
    }
    if target.kind == TargetKind::RemoteDesktop || multi_line || text.chars().count() > MAX_TYPED_CHARS {
        return Strategy::Paste;
    }
    Strategy::Type
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(kind: TargetKind) -> Target {
        Target {
            kind,
            elevated_mismatch: false,
        }
    }

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
    fn strips_escape_sequences_used_for_terminal_injection() {
        assert_eq!(sanitize_for_insertion("ok\u{1b}[31mred\u{1b}[0m"), "ok[31mred[0m");
        assert_eq!(sanitize_for_insertion("a\u{8}\u{7f}b"), "ab");
    }

    #[test]
    fn keeps_plain_text_unchanged() {
        assert_eq!(sanitize_for_insertion("just text"), "just text");
    }

    #[test]
    fn short_single_line_is_typed() {
        assert_eq!(choose_strategy("hello world", &target(TargetKind::Normal)), Strategy::Type);
        assert_eq!(choose_strategy("ls -la", &target(TargetKind::Terminal)), Strategy::Type);
    }

    #[test]
    fn multi_line_and_long_text_is_pasted() {
        assert_eq!(choose_strategy("a\nb", &target(TargetKind::Normal)), Strategy::Paste);
        let long = "x".repeat(MAX_TYPED_CHARS + 1);
        assert_eq!(choose_strategy(&long, &target(TargetKind::Normal)), Strategy::Paste);
    }

    #[test]
    fn remote_desktop_uses_paste() {
        assert_eq!(choose_strategy("hi", &target(TargetKind::RemoteDesktop)), Strategy::Paste);
    }

    #[test]
    fn multi_line_into_terminal_is_copy_only() {
        assert!(matches!(
            choose_strategy("echo a\nrm -rf x", &target(TargetKind::Terminal)),
            Strategy::CopyOnly(m) if m.contains("terminal")
        ));
    }

    #[test]
    fn elevated_target_is_copy_only_with_reason() {
        let t = Target {
            kind: TargetKind::Normal,
            elevated_mismatch: true,
        };
        assert!(matches!(choose_strategy("hi", &t), Strategy::CopyOnly(m) if m.contains("administrator")));
    }

    #[test]
    fn no_focus_or_own_window_is_copy_only() {
        assert!(matches!(choose_strategy("hi", &target(TargetKind::None)), Strategy::CopyOnly(_)));
        assert!(matches!(choose_strategy("hi", &target(TargetKind::OwnWindow)), Strategy::CopyOnly(_)));
    }

    #[test]
    fn unsupported_platform_never_reports_inserted() {
        assert_eq!(UnsupportedInserter.insert("x").outcome, InsertOutcome::Failed);
    }
}
