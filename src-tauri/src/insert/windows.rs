//! Windows insertion adapter: SendInput Unicode typing, clipboard paste with save/restore, and a
//! copy-only fallback. Outcomes are what the OS reported: "inserted" means the keystrokes were
//! accepted into the focused window's input queue (Windows cannot tell us whether the app used them).

use std::thread::sleep;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, GlobalFree, HANDLE, HWND};
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber,
    OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId};

use super::{choose_strategy, InsertOutcome, InsertResult, Strategy, Target, TargetKind, TextInserter};

const VK_V: u16 = 0x56;
/// How long the target gets to read the clipboard after the paste shortcut before we restore it.
const PASTE_SETTLE: Duration = Duration::from_millis(450);
/// The hotkey's modifiers may still be down when we finish; typing then would trigger shortcuts.
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct WindowsInserter;

impl WindowsInserter {
    pub fn new() -> Self {
        Self
    }
}

impl TextInserter for WindowsInserter {
    fn insert(&mut self, text: &str) -> InsertResult {
        if text.is_empty() {
            return InsertResult::failed("Nothing to insert.");
        }
        let target = inspect_foreground();
        let strategy = choose_strategy(text, &target);
        if !matches!(strategy, Strategy::CopyOnly(_)) && !wait_for_modifiers_released() {
            return copy_only(text, "Modifier keys were still held, so the text was copied instead of typed.");
        }
        match strategy {
            Strategy::Type => match type_unicode(text) {
                Ok(()) => inserted("typed"),
                Err(e) => paste_or_copy(text, Some(e)),
            },
            Strategy::Paste => paste_or_copy(text, None),
            Strategy::CopyOnly(reason) => copy_only(text, &reason),
        }
    }

    fn describe(&self) -> &'static str {
        "windows (SendInput / clipboard)"
    }
}

fn inserted(method: &str) -> InsertResult {
    InsertResult {
        outcome: InsertOutcome::Inserted,
        method: method.into(),
        message: None,
    }
}

fn paste_or_copy(text: &str, prior_error: Option<String>) -> InsertResult {
    match paste(text) {
        Ok(()) => inserted("clipboard-paste"),
        Err(e) => {
            let why = prior_error.map(|p| format!("{p}; {e}")).unwrap_or(e);
            copy_only(text, &format!("Could not insert automatically ({why}). The text was copied; paste it with Ctrl+V."))
        }
    }
}

pub(super) fn copy_to_clipboard(text: &str) -> Result<(), String> {
    with_clipboard(|| set_text(text))
}

fn copy_only(text: &str, reason: &str) -> InsertResult {
    match with_clipboard(|| set_text(text)) {
        Ok(()) => InsertResult {
            outcome: InsertOutcome::CopiedOnly,
            method: "clipboard-copy".into(),
            message: Some(reason.into()),
        },
        Err(e) => InsertResult::failed(format!("{reason} Copying also failed: {e}. The transcript is in History.")),
    }
}

// ---------- target inspection ----------

fn inspect_foreground() -> Target {
    // SAFETY: plain Win32 queries on the current foreground window; handles are closed below.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return Target { kind: TargetKind::None, elevated_mismatch: false };
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId() {
            return Target { kind: TargetKind::OwnWindow, elevated_mismatch: false };
        }
        let kind = classify_window_class(&window_class(hwnd));
        let elevated_mismatch = !process_elevated(GetCurrentProcess()).unwrap_or(false) && {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                // Being refused even limited query rights is itself a strong sign of elevation.
                true
            } else {
                let e = process_elevated(h).unwrap_or(false);
                CloseHandle(h);
                e
            }
        };
        Target { kind, elevated_mismatch }
    }
}

unsafe fn window_class(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Known window classes for console hosts, terminal emulators, and remote-desktop clients.
pub(crate) fn classify_window_class(class: &str) -> TargetKind {
    const TERMINALS: &[&str] = &[
        "ConsoleWindowClass",             // conhost (cmd, PowerShell)
        "CASCADIA_HOSTING_WINDOW_CLASS",  // Windows Terminal
        "mintty",                         // Git Bash / MSYS2 / Cygwin
        "PuTTY",
        "VirtualConsoleClass",            // ConEmu / Cmder
        "org.wezfurlong.wezterm",
        "Alacritty",
    ];
    const REMOTE: &[&str] = &["TscShellContainerClass", "RAIL_WINDOW", "vncviewer"];
    if TERMINALS.iter().any(|t| class.eq_ignore_ascii_case(t)) {
        TargetKind::Terminal
    } else if REMOTE.iter().any(|t| class.eq_ignore_ascii_case(t)) {
        TargetKind::RemoteDesktop
    } else {
        TargetKind::Normal
    }
}

unsafe fn process_elevated(process: HANDLE) -> Option<bool> {
    let mut token: HANDLE = std::ptr::null_mut();
    if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
        return None;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut len = 0u32;
    let ok = GetTokenInformation(
        token,
        TokenElevation,
        &mut elevation as *mut _ as *mut _,
        std::mem::size_of::<TOKEN_ELEVATION>() as u32,
        &mut len,
    );
    CloseHandle(token);
    (ok != 0).then_some(elevation.TokenIsElevated != 0)
}

fn wait_for_modifiers_released() -> bool {
    let deadline = Instant::now() + MODIFIER_RELEASE_TIMEOUT;
    loop {
        // SAFETY: GetAsyncKeyState has no preconditions.
        let held = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            .any(|vk| unsafe { GetAsyncKeyState(*vk as i32) } as u16 & 0x8000 != 0);
        if !held {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        sleep(Duration::from_millis(15));
    }
}

// ---------- keystrokes ----------

fn key(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), String> {
    // SAFETY: `inputs` is a valid slice of INPUT structs for the duration of the call.
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(format!("Windows accepted {sent} of {} key events", inputs.len()))
    }
}

/// Type text as Unicode key events (UTF-16 units, so surrogate pairs work). Newlines never reach
/// here (multi-line text is pasted), so no Enter key is ever synthesized.
fn type_unicode(text: &str) -> Result<(), String> {
    let mut inputs = Vec::with_capacity(text.len() * 2);
    for unit in text.encode_utf16() {
        if unit == '\n' as u16 || unit == '\r' as u16 {
            return Err("line breaks are never typed".into());
        }
        inputs.push(key(0, unit, KEYEVENTF_UNICODE));
        inputs.push(key(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    // Chunk so very long bursts don't overflow slow apps' input queues.
    for chunk in inputs.chunks(200) {
        send(chunk)?;
        sleep(Duration::from_millis(2));
    }
    Ok(())
}

fn send_paste_shortcut() -> Result<(), String> {
    send(&[
        key(VK_CONTROL, 0, 0),
        key(VK_V, 0, 0),
        key(VK_V, 0, KEYEVENTF_KEYUP),
        key(VK_CONTROL, 0, KEYEVENTF_KEYUP),
    ])
}

// ---------- clipboard ----------

/// Open the clipboard (retrying briefly: other apps hold it for short periods), run `f`, close.
fn with_clipboard<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let deadline = Instant::now() + Duration::from_millis(500);
    // SAFETY: Open/CloseClipboard are paired; `f` runs only while the clipboard is open.
    unsafe {
        while OpenClipboard(std::ptr::null_mut()) == 0 {
            if Instant::now() >= deadline {
                return Err("the clipboard is busy".into());
            }
            sleep(Duration::from_millis(20));
        }
        let r = f();
        CloseClipboard();
        r
    }
}

/// Snapshot of every memory-backed clipboard format, so a paste can put things back exactly.
struct ClipboardSnapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

/// GDI-handle formats are not HGLOBAL memory; Windows re-synthesizes them from CF_DIB/text.
fn is_handle_format(fmt: u32) -> bool {
    matches!(fmt, 2 /*BITMAP*/ | 3 /*METAFILEPICT*/ | 9 /*PALETTE*/ | 14 /*ENHMETAFILE*/
        | 0x80 /*OWNERDISPLAY*/ | 0x82 /*DSPBITMAP*/ | 0x83 /*DSPMETAFILEPICT*/ | 0x8E /*DSPENHMETAFILE*/)
}

unsafe fn snapshot() -> ClipboardSnapshot {
    let mut formats = Vec::new();
    let mut fmt = EnumClipboardFormats(0);
    while fmt != 0 {
        if !is_handle_format(fmt) {
            let h = GetClipboardData(fmt);
            if !h.is_null() {
                let size = GlobalSize(h);
                let p = GlobalLock(h) as *const u8;
                if !p.is_null() {
                    formats.push((fmt, std::slice::from_raw_parts(p, size).to_vec()));
                    GlobalUnlock(h);
                }
            }
        }
        fmt = EnumClipboardFormats(fmt);
    }
    ClipboardSnapshot { formats }
}

unsafe fn put_bytes(fmt: u32, bytes: &[u8]) -> Result<(), String> {
    let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
    if h.is_null() {
        return Err("out of memory".into());
    }
    let p = GlobalLock(h) as *mut u8;
    if p.is_null() {
        GlobalFree(h);
        return Err("could not lock clipboard memory".into());
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
    GlobalUnlock(h);
    if SetClipboardData(fmt, h).is_null() {
        GlobalFree(h); // ownership was not transferred
        return Err("SetClipboardData failed".into());
    }
    Ok(())
}

fn utf16z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Replace the clipboard with `text`, marked so Windows clipboard history and cloud sync skip it
/// (dictated text may be private).
fn set_text(text: &str) -> Result<(), String> {
    // SAFETY: called inside `with_clipboard`.
    unsafe {
        EmptyClipboard();
        let wide: Vec<u8> = utf16z(text).iter().flat_map(|u| u.to_le_bytes()).collect();
        put_bytes(CF_UNICODETEXT as u32, &wide)?;
        for name in ["ExcludeClipboardContentFromMonitorProcessing", "CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"] {
            let fmt = RegisterClipboardFormatW(utf16z(name).as_ptr());
            if fmt != 0 {
                let _ = put_bytes(fmt, &0u32.to_le_bytes());
            }
        }
        Ok(())
    }
}

fn paste(text: &str) -> Result<(), String> {
    let saved = with_clipboard(|| {
        // SAFETY: inside `with_clipboard`.
        let snap = unsafe { snapshot() };
        set_text(text)?;
        Ok(snap)
    })?;
    // SAFETY: no preconditions.
    let ours = unsafe { GetClipboardSequenceNumber() };
    let sent = send_paste_shortcut();
    sleep(PASTE_SETTLE);
    restore_if_unchanged(saved, ours);
    sent
}

/// Put the user's clipboard back, unless something else changed it after we set ours (then the
/// newer content wins and we leave it alone).
fn restore_if_unchanged(saved: ClipboardSnapshot, our_sequence: u32) {
    // SAFETY: no preconditions.
    if unsafe { GetClipboardSequenceNumber() } != our_sequence {
        return;
    }
    let _ = with_clipboard(|| {
        // SAFETY: inside `with_clipboard`.
        unsafe {
            EmptyClipboard();
            for (fmt, bytes) in &saved.formats {
                let _ = put_bytes(*fmt, bytes);
            }
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_terminals_and_remote_desktop() {
        assert_eq!(classify_window_class("ConsoleWindowClass"), TargetKind::Terminal);
        assert_eq!(classify_window_class("CASCADIA_HOSTING_WINDOW_CLASS"), TargetKind::Terminal);
        assert_eq!(classify_window_class("mintty"), TargetKind::Terminal);
        assert_eq!(classify_window_class("TscShellContainerClass"), TargetKind::RemoteDesktop);
        assert_eq!(classify_window_class("Notepad"), TargetKind::Normal);
        assert_eq!(classify_window_class("Chrome_WidgetWin_1"), TargetKind::Normal);
    }

    #[test]
    fn handle_formats_are_skipped_in_snapshots() {
        assert!(is_handle_format(2));
        assert!(is_handle_format(14));
        assert!(!is_handle_format(CF_UNICODETEXT as u32));
        assert!(!is_handle_format(8)); // CF_DIB is memory-backed and restorable
    }

    #[test]
    fn utf16z_is_nul_terminated() {
        assert_eq!(utf16z("hi"), vec![b'h' as u16, b'i' as u16, 0]);
    }

    /// Exercises the real clipboard: save → set ours → restore. Ignored by default because it
    /// briefly replaces the clipboard (restored at the end). Run: cargo test -- --ignored clipboard
    #[test]
    #[ignore]
    fn clipboard_round_trip_restores_previous_text() {
        let developers = with_clipboard(|| Ok(unsafe { snapshot() })).unwrap();
        with_clipboard(|| set_text("voxelle-previous")).unwrap();
        let saved = with_clipboard(|| {
            let s = unsafe { snapshot() };
            set_text("voxelle-transcript")?;
            Ok(s)
        })
        .unwrap();
        let seq = unsafe { GetClipboardSequenceNumber() };
        restore_if_unchanged(saved, seq);
        let restored = with_clipboard(|| unsafe {
            let h = GetClipboardData(CF_UNICODETEXT as u32);
            let p = GlobalLock(h) as *const u16;
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
            GlobalUnlock(h);
            Ok(s)
        })
        .unwrap();
        // Put the developer's clipboard back before asserting.
        restore_if_unchanged(developers, unsafe { GetClipboardSequenceNumber() });
        assert_eq!(restored, "voxelle-previous");
    }
}
