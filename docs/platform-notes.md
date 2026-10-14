# Platform notes

Per-platform constraints for the dictation loop (P0 §0.1 requires these documented). "verified" means
observed on this machine; otherwise "to verify on <platform>".

## Windows (primary dev target)
- **Global hotkeys (implemented):** a `WH_KEYBOARD_LL` low-level hook on a dedicated thread
  (`src-tauri/src/hotkeys/windows.rs`). It sees key-up (push-to-talk), ignores injected events
  (`LLKHF_INJECTED`, so our own typing/paste never triggers it), and swallows only the bound keys.
  A registered hotkey (`RegisterHotKey` / Tauri global-shortcut) was not used: press-only.
  Limitation: LL hooks do not receive input aimed at elevated windows when Voxelle is not elevated,
  and Windows removes a hook that stalls (>~1 s); the hook does no blocking work.
- **Text insertion (implemented):** SendInput Unicode typing → clipboard paste (Ctrl+V) with full
  clipboard save/restore → copy-only. **Elevation boundary:** a non-elevated process cannot
  `SendInput` into an elevated window (UIPI); we detect the target's token elevation and report
  **copied-only** with the reason. Terminals (conhost, Windows Terminal, mintty, …) never receive
  multi-line text. Remote-desktop clients (mstsc) get clipboard paste. "Inserted" = Windows accepted
  all key events into the focused window's queue; the OS cannot confirm the app consumed them.
  **Not detectable yet:** password/secure fields (would need UI Automation `IsPassword`), apps that
  drop synthetic input (some games, some anti-cheat). Secure Desktop (UAC, lock screen) blocks
  everything — the hook and insertion simply don't run there.
- **Clipboard privacy:** clipboard writes are tagged to stay out of Win+V history and cloud
  clipboard (`CanIncludeInClipboardHistory=0`, `CanUploadToCloudClipboard=0`,
  `ExcludeClipboardContentFromMonitorProcessing`).
- **Mic permission:** Settings › Privacy › Microphone › "Let desktop apps access your microphone".
  When denied, cpal returns an error that we surface as a clear message (not a crash).
- **Recording indicator:** non-focusable (`WS_EX_NOACTIVATE`), click-through, always-on-top overlay
  window at the bottom centre of the primary monitor's work area.
- **GPU:** whisper.cpp Vulkan backend (`--features gpu-vulkan`), verified on GTX 1080 / driver 582.28 /
  Vulkan 1.4.312. The Vulkan build imports `vulkan-1.dll`; machines without a Vulkan loader (some
  VMs) need the CPU build until delay-loading is added.
- **Building:** run cargo/tauri through `scripts\win-env.cmd` (Ninja + VS dev env). The default VS
  generator path produces an unoptimized whisper.cpp (~6x slower) — see decisions 2026-10-14.
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
