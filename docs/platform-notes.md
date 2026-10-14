# Platform notes

Per-platform constraints for the dictation loop (P0 §0.1 requires these documented). "verified" means
observed on this machine; otherwise "to verify on <platform>".

## Windows (primary dev target)
- **Global hotkeys:** Tauri global-shortcut registers accelerators that fire on **key-down** only —
  enough for toggle/cancel, **not** for true push-to-talk (needs key-up). For hold-to-talk we add a
  low-level key-state layer (candidate: `rdev` low-level hook, or Win32 `SetWindowsHookEx`/
  `RegisterRawInputDevices` via `windows-rs`). Decision recorded in decisions.md when implemented.
- **Text insertion:** primary = set clipboard → synthesize Ctrl+V (`SendInput`); verify via UI
  Automation where available; fallback = copied-only. **Elevation boundary:** a non-elevated process
  cannot `SendInput` into an elevated (admin) window (UIPI). Detect and report "copied-only" rather
  than silently failing. Secure Desktop (UAC prompt, lock screen) and some password fields block
  injection — report, never claim "inserted".
- **Mic permission:** Windows 10/11 has a per-app microphone privacy setting; capture can fail with a
  permission error that must surface clearly (not a crash).
- **Packaging:** Tauri NSIS/MSI installer; WebView2 runtime required (present: 154.x).

## Linux
- **X11:** global shortcuts and synthetic input (XTEST) generally work; clipboard via X selections.
- **Wayland:** **no universal global-shortcut API.** Options, in order of portability:
  1. `org.freedesktop.portal.GlobalShortcuts` XDG portal (newer desktops; user-approved) — preferred.
  2. Compositor-specific config (GNOME/KDE custom shortcut invoking the app, or wlroots protocols).
  3. Fallback: in-app/tray toggle + document that global PTT is **platform-limited** on Wayland.
  Synthetic input is also restricted on Wayland; insertion may be **clipboard + manual paste** there.
  We label this **platform-limited**, never fake it.
- **Text insertion:** X11 = clipboard + XTEST paste; Wayland = `wtype`/portal where available, else
  copied-only.

## macOS
- **Permissions required:** Microphone (TCC) for capture; **Accessibility** for synthetic keystrokes
  (paste) and global key monitoring; **Input Monitoring** for low-level key-release detection
  (push-to-talk). App must detect missing grants and link the user to System Settings, not crash.
- **Global hotkeys:** Carbon `RegisterEventHotKey` / `CGEventTap` (via a crate) for key-down and
  key-up. `CGEventTap` key monitoring needs Accessibility/Input Monitoring.
- **Text insertion:** clipboard + `Cmd+V` via `CGEvent`; AX API (`AXUIElement`) for focused-field
  context/verification. Secure input fields (password) block synthetic events — report copied-only.

## Cross-platform rule
The recognizer, state machine, storage, and policy engine are platform-agnostic Rust. Only the
hotkey, audio device, text-insertion, and permission layers are platform adapters behind traits
(`HotkeyBackend`, `AudioCapture`, `TextInserter`, `PermissionProbe`). A capability that cannot be
honored on a platform is reported as **platform-limited / unavailable**, never silently degraded into
a false success.
