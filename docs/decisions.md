# Decisions log

Append-only record of non-obvious choices. Format: date — decision — why — alternatives considered.

---

## 2026-10-13 — Stack: Tauri 2 + Rust core + React/TypeScript UI
- **Decision:** Adopt the documented default stack. Desktop shell = Tauri 2; core logic = Rust;
  UI = React + TypeScript + Vite. Local ASR = whisper.cpp via Rust bindings (`whisper-rs`) with a
  sidecar-binary fallback. Audio capture = `cpal`. Storage = SQLite (`rusqlite` with bundled
  SQLite) using migrations + parameterized queries. Global hotkeys = Tauri global-shortcut for the
  toggle path, plus a dedicated key-state layer for true push-to-talk key-release detection
  (`rdev`/platform hooks — final choice recorded when implemented in 0.1).
- **Why:** The repo was empty (no stack to preserve — see Audit below). The default stack gives the
  renderer **no Node/OS access** (security.md §6), a small native footprint vs Electron, and a
  first-class Rust boundary for the policy engine (security.md §2). It also contrasts cleanly with
  the reference app (Electron + cloud), which is a branding/originality goal.
- **Alternatives considered:** (a) Electron + prebuilt whisper sidecar — buildable here without a
  C++ toolchain, but weaker renderer isolation, heavier, and converges with the reference app's
  architecture. (b) GNU (MinGW) Rust toolchain to dodge MSVC — Tauri 2 on Windows is only
  first-class on MSVC; rejected. The user chose to install MSVC Build Tools and keep the default
  stack.

## 2026-10-13 — Audit result: repository is empty of application code
- **Decision:** Treated as a greenfield project; picked the default stack (above) rather than
  preserving an existing one.
- **Evidence:** Before any scaffold, the repo contained only `CLAUDE.md`, `assets/` (one logo SVG),
  `prompts/`, and `docs/`. No `package.json`, `Cargo.toml`, `src/`, lockfiles, or tests.
  `git log` had zero commits.

## 2026-10-13 — BLOCKER (environment): MSVC C++ Build Tools not installed
- **Status:** Blocking P0 completion. User is installing (chose "Install MSVC Build Tools").
- **Finding:** `cargo build` on a trivial crate fails to link — the only `link.exe` on PATH is Git's
  GNU coreutils `link`. No `cl.exe`, no `vswhere.exe`, no MSVC `link.exe` found under Program Files.
  The Rust toolchain is `x86_64-pc-windows-msvc` (host), which requires the MSVC linker + CRT.
  whisper.cpp also needs a C++ compiler. So neither the Rust core nor the ASR engine can compile
  until Build Tools are present.
- **Remediation given to user:**
  `winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait
  --add Microsoft.VisualStudio.Workload.VCTools --add
  Microsoft.VisualStudio.Component.Windows11SDK.22621 --includeRecommended"`
- **Impact on this session:** All non-compiling work proceeds (docs, branding, scaffold, React/TS UI
  which builds on Node alone, Rust source authored but not yet compiled). P0 cannot reach its
  definition of done (which requires a verified end-to-end offline transcript) until the toolchain is
  installed and the Rust build + a whisper model run on this machine.

## 2026-10-13 — Verified environment baseline
- Node v24.11.1, npm 11.6.2 (no pnpm → use npm). Rust/cargo 1.96.0 (MSVC host). CMake 4.2.1.
  WebView2 runtime 154.x present. GPU: NVIDIA GTX 1080, 8192 MiB VRAM, driver 582.28. Git 2.56.
  These satisfy the Node-side build and (once Build Tools land) the Rust/whisper build and optional
  CUDA acceleration path.

## 2026-10-13 — Repo license: Apache-2.0
- **Decision:** License the repository under Apache-2.0 (LICENSE + NOTICE added; declared in
  package.json and Cargo.toml). **Why:** permissive (satisfies the non-negotiable) and adds an
  explicit patent grant, which suits a project that may bundle/download models. Compatible with our
  MIT/Apache/BSD dependencies. **Alternative:** MIT (simpler, no patent clause) — viable, but the
  patent grant is worth the extra length. README is public-safe and does not mention the reference app.

## 2026-10-13 — Reference survey findings fold into P0 plan
- Source: docs/reference-survey.md (surveyed Handy, OpenWhispr, Whispering, WhisperWriter).
- **Push-to-talk key-release:** confirmed a press-only shortcut API is insufficient; the proven
  fit for our Tauri+Rust stack is an in-process low-level key hook on a dedicated thread (`rdev`,
  as in Handy's `handy-keys`). On Windows read key-up from the hook event, not `GetAsyncKeyState`.
  Locks in the `rdev` candidate from dependencies.md for the hotkey slice.
- **GPU on the GTX 1080:** prefer whisper.cpp **Vulkan** over CUDA — avoids a hard CUDA dependency
  and still accelerates on the 1080. Always keep the CPU fallback and benchmark before defaulting
  (P0 §0.2). Recorded here so the ASR slice evaluates Vulkan first.
- **Text insertion:** clipboard + synthetic Ctrl/Cmd+V (save/restore clipboard) on Windows/macOS;
  Linux needs runtime branching (Wayland `wtype`/`wl-copy` vs X11 `xdotool`) with a user override —
  matches platform-notes.md.
- **Copy policy:** Handy and OpenWhispr are **MIT** (copyable with attribution + NOTICE per
  CLAUDE.md, if we ever lift a snippet); Whispering (AGPL) and WhisperWriter (GPL) are **learn-only**
  — reimplement, never copy. No code copied this session.

## 2026-10-13 — Branding file organized
- Moved the stray `assets/Untitled Design.svg` to `assets/branding/voxelle-logo-master.svg`
  (content untouched) to match CLAUDE.md's expected `assets/branding/` location. Originals are not
  edited; all icon variants will be derived from this master.

## 2026-10-14 — BLOCKER RESOLVED: MSVC toolchain present
- User installed Visual Studio C++ tools (VS 18, MSVC 14.51). `cargo build` of the core succeeded
  first try (13 dead-code warnings), `cargo test` 15/15, `npm run tauri dev` launched the window and
  IPC round-tripped ("Core connected").

## 2026-10-14 — whisper-rs needs libclang on Windows → installed LLVM
- **Finding:** whisper-rs-sys' bundled `bindings.rs` is generated on Linux; on Windows its layout
  asserts fail (`_G_fpos_t`, `_IO_FILE` sizes). Bindings must be generated by bindgen → libclang.
- **Decision:** installed LLVM 23.1.3 (winget, build-time only), `LIBCLANG_PATH` set for the user.
  **Alternative:** commit pre-generated Windows bindings — rejected (would drift from the crate).

## 2026-10-14 — Windows native builds go through Ninja (`scripts/win-env.cmd`)
- **Finding 1 (performance bug):** with the default Visual Studio CMake generator, the `cmake` crate
  overwrites `CMAKE_C/CXX_FLAGS_RELEASE` with cc's flags minus `/O*`, so whisper.cpp compiled
  **without optimization** (`-nologo -MD -Brepro -W0`). Measured impact: base model 10.2 s → 1.78 s
  per ~4.5 s clip on CPU after the fix. Ninja builds get CMake's `/O2 /Ob2`.
- **Finding 2:** the Vulkan shader-generator sub-build exceeds MAX_PATH (260) under MSBuild even
  from `C:\vxb`. Ninja + short `CARGO_TARGET_DIR` (`%USERPROFILE%\.voxelle-target`) fixes it.
- **Decision:** `scripts/win-env.cmd <cmd>` sets up vcvars64 + Ninja + libclang + Vulkan SDK path +
  short target dir. All Windows builds (cargo, `npm run tauri dev/build`) should run through it.
- Leftover: an empty/failed build dir `C:\vxb` from the first attempt — deletion was blocked by the
  agent sandbox; safe for the user to delete.

## 2026-10-14 — whisper-rs abort callback: do not use `set_abort_callback_safe`
- **Finding:** whisper-rs 0.16 boxes the closure as `Box<Box<dyn FnMut>>` but its trampoline casts
  the pointer to the concrete closure type — type confusion; the hook returned garbage and aborted
  decoding (whisper error -9 on every clip).
- **Decision:** register a plain `extern "C"` hook reading our `CancelToken`'s `AtomicBool`
  (`asr/whisper.rs`). Cancellation verified separately by test (see feature-status).

## 2026-10-14 — GPU path: Vulkan (evaluated on the GTX 1080), CPU fallback kept
- **Evaluated:** whisper.cpp Vulkan backend vs CPU (Xeon W-2235, 6C/12T, AVX-512, 6 threads) on the
  12-clip corpus (`bench/corpus`, ~57 s of speech). Results in `bench/results/asr-latest.jsonl` and
  the table in docs/feature-status.md. Vulkan is 12–21× faster per clip on every model, with
  identical WER, and uses 0.3–1.4 GB VRAM — well inside the 1080's 8 GB.
- **Decision:** build with `--features gpu-vulkan` on Windows; `preferGpu` setting chooses at
  runtime; a GPU load failure falls back to CPU automatically (`WhisperBackend::load`).
- **CUDA not evaluated:** CUDA 12.4 is installed, but Vulkan already meets the latency target with
  margin and avoids a CUDA runtime dependency for users; a CUDA build would need a ~20-minute
  compile for marginal gain. Revisit in P9 if needed.
- **Open risk:** a Vulkan build imports `vulkan-1.dll` at load time, so it will not start on a
  machine without a Vulkan loader (rare on real hardware, common in VMs). Before packaging: delay-load
  the DLL and probe it before enabling the GPU, or ship CPU + GPU builds. Tracked in feature-status.

## 2026-10-14 — Latency target and default model
- **Target:** stop → transcript **≤ 1.0 s (p50) for a ~5 s utterance** on the user's machine. Under
  ~1 s dictation feels immediate; above ~2 s users start waiting on it.
- **Rule:** recommend the largest installed model whose measured p50 meets the target; if none
  does, recommend the fastest and say it misses the target. Users can always override.
- **This machine:** see the GPU entry above for numbers; the recommendation is recorded with them.

## 2026-10-14 — Model file format: legacy GGML `.bin` is allowed for whisper.cpp
- security.md §6 lists GGUF/safetensors/ONNX. whisper.cpp's published models use the older GGML
  container (`.bin`, magic `ggml`), which is the same family as GGUF: raw tensors + hyperparameters,
  no code, no pickle. **Decision:** accept it only (a) from the pinned catalog, (b) with the GGML
  magic bytes checked before the native loader sees the file, and (c) after SHA-256 matches the
  pinned value. Any other `.bin` is rejected.

## 2026-10-14 — Text insertion (Windows)
- **Ladder:** SendInput Unicode typing (clipboard untouched) → clipboard paste with full
  save/restore → copy-only with a reason. Pure `choose_strategy` decides from the target:
  multi-line or >800 chars or remote-desktop → paste; elevated target (UIPI) / no focus / Voxelle
  focused / multi-line into a terminal → copy-only. Line breaks are never typed (no auto-submit).
- **Clipboard privacy:** our clipboard writes carry `ExcludeClipboardContentFromMonitorProcessing`,
  `CanIncludeInClipboardHistory=0`, `CanUploadToCloudClipboard=0`, so dictated text stays out of
  Win+V history and cloud clipboard sync.
- **Restore safety:** restore only if the clipboard sequence number is unchanged after the paste
  settle window (450 ms); if something else changed it, the newer content wins.
- **Honesty:** "inserted" means Windows accepted every synthetic key event into the focused window's
  queue; Windows cannot confirm the app consumed them. Documented in platform-notes.

## 2026-10-14 — Global hotkeys (Windows): own WH_KEYBOARD_LL hook, not rdev/global-shortcut
- Low-level hook on a dedicated thread gives key-up (push-to-talk), ignores injected events (so our
  own typing can never trigger it), and lets us swallow only the bound keys. Matching is a pure,
  tested state machine. The hook never takes a lock that dictation holds (Windows silently removes
  LL hooks that stall); it reads a lock-free `session_active` flag.
- **Defaults:** toggle `Ctrl+Shift+Space`, push-to-talk `RightCtrl` (hold), cancel `Escape` (only
  swallowed while a session is active). A modifier-only PTT key used in a chord (RightCtrl+C)
  cancels the dictation quietly instead of recording. Taps under 250 ms are cancelled.

## 2026-10-15 — Audio capture: one process-lifetime audio thread, concrete default endpoint
- **Bug (user-reported, reproduced):** "audio capture not available: Failed to get audio client:
  Cannot change thread mode after it is set (RPC_E_CHANGED_MODE)" after 2–3 recordings, then on
  every one. `examples/capture_repeat.rs` reproduced it on round 3–4.
- **Cause found:** cpal opens the *virtual* default capture device through
  `ActivateAudioInterfaceAsync`; on this machine that started failing after a few activations and
  never recovered. It also cost ~0.6 s of missing audio at the start of every recording. Opening the
  concrete endpoint (`IMMDevice::Activate`) has neither problem: 20/20 rounds, full-length audio.
- **Decision:** resolve "System default" to the concrete endpoint id each time a recording opens
  (so it still follows the user's default). All cpal calls run on one process-lifetime thread that
  joins the MTA first, because cpal caches a global device enumerator in whichever apartment first
  touches audio; a short-lived thread would leave it dangling.

## 2026-10-15 — "Keep microphone ready" (off by default)
- Bluetooth headsets switch to the hands-free profile when their mic opens and deliver little or no
  audio for ~0.5–3 s, so the first words are lost. The only full fix is to not reopen the mic.
- **Decision:** optional setting (Off / 1 / 5 / 30 min / until quit), **default Off** (privacy: the
  mic indicator stays on while open, and BT headsets stay in call-quality audio). Between
  recordings the callback drops samples; nothing is buffered. Cancel/errors always release the mic.
- The recording pill says "Starting mic…" (amber) until the device delivers its first samples, and
  the dashboard suggests the setting when first audio took ≥ 400 ms.

## 2026-10-15 — Clipboard: deliberate copies are ordinary entries, verified by read-back
- Supersedes the clipboard-privacy bullet of 2026-10-14 for *deliberate* copies. The user reported
  that Copy "says it copied but didn't". Copy itself worked (verified by reading back from another
  process), but every write was flagged out of Win+V clipboard history, which looks like a failed
  copy to anyone who checks there.
- **Decision:** Copy buttons and the copy-only fallback write a normal clipboard entry and read it
  back before reporting success. Only the transient paste buffer (restored ~450 ms later) keeps the
  history/cloud/monitor exclusion flags.

## 2026-10-15 — Shortcuts are recorded by pressing keys (incl. Win), not typed
- User feedback: typing shortcut names is clumsy, and Win/Ctrl+Win could not be configured.
- **Decision:** a recorder in the core's existing WH_KEYBOARD_LL hook. While recording, every
  physical key goes to a pure state machine (`hotkeys/capture.rs`, tested) and is swallowed, so Win
  does not open Start. The shortcut is the largest set of keys held at once, finalized on release.
  Recording ends itself after 15 s so a lost UI can never leave the keyboard swallowed.
- **New binding kind:** modifier chords (2+ modifiers, e.g. Ctrl+Win), usable for push-to-talk,
  toggle (fires on clean release), and cancel. Pressing another key while the chord is held
  (Ctrl+Win+Right = switch desktop) cancels quietly and the system shortcut still works. A single
  generic modifier ("Ctrl") stays rejected; single modifiers are side-specific (RightCtrl).
- **Start-menu masking:** when a binding that leaves Win held fires, we tap an unassigned virtual
  key (0xE8) from a short-lived thread so releasing Win does not open Start (our hook ignores
  injected keys, so the mask can never trigger anything).

## 2026-10-15 — Tray + close-to-tray + single instance
- Closing the window hides it (setting "Keep running in the tray when closed", **default on**, as
  the user asked); shortcuts keep working. Tray: left-click opens; menu = Open / Start-stop
  dictation / Quit. A second launch shows the running instance (tauri-plugin-single-instance)
  instead of installing a second keyboard hook. Tray icon = the app icon for now; a dedicated small
  monochrome tray glyph is a pending branding item (see ui-direction).

## 2026-10-17 — Which keys may be a global shortcut (our rules)
- Root cause of the user's "keyboard got messed up": the recorder accepted any key. Their saved
  settings held push-to-talk = **LeftWin** (every Win press started a dictation and Start was
  masked) and hands-free = Ctrl+Space. A plain key (letters, arrows) would have been swallowed in
  every app.
- **Decision:** one rule set, `hotkeys::check_allowed(binding, role)`, applied by the recorder and
  by every settings save (and at startup):
  - typing keys (letters, digits, punctuation, Space, numpad) need **two** modifiers;
  - editing/navigation keys (arrows, Home/End, PgUp/PgDn, Insert, Delete, Escape, PrintScreen,
    Menu) need two modifiers — except for **Cancel**, which only acts while dictating;
  - F1–F24, Pause, ScrollLock, CapsLock are fine alone; single side-specific Ctrl/Alt/Shift are fine;
  - the Win key alone is refused (it opens Start); Ctrl+Alt (AltGr), Alt+Shift and Ctrl+Shift
    (layout switching) chords are refused; a short list of Windows' own shortcuts is refused;
  - no two roles may share keys.
- Saved shortcuts that break these rules are reset to their default at startup (or turned off if
  the default collides) and the change is logged; the others keep working. Previously one bad
  value disabled every shortcut silently.
- Refusals are shown in a dialog with the reason; accepted shortcuts are confirmed with a toast.
  Wording is our own.

## 2026-10-17 — Recorder: never strand a key, quiet timeout, emit off the hook thread
- The recorder only swallows key-ups of keys whose key-down it took (a key held before recording
  started is released normally), and key-ups still owed when recording ends are swallowed by a
  drain list, so neither Windows nor the shortcut matcher sees half a key press.
- The timeout counts from the last key event (never while a key is held) and ends quietly
  (`ended` → "Stopped listening"), not with an error. Esc alone backs out (except for Cancel,
  where Esc is a valid choice).
- Updates are queued from the hook procedure and emitted on another thread: Windows silently
  removes a low-level hook that is slow to return, which would kill every shortcut.

## 2026-10-17 — Bundle Inter + JetBrains Mono; custom dropdown
- User asked for a nicer, readable font. v2 used the platform font; this replaces it with Inter
  (UI) and JetBrains Mono (keys, numbers, paths), both SIL OFL 1.1, bundled via npm/Vite (no
  network). Same look on Windows/macOS/Linux.
- Native `<select>` popups are OS-drawn square lists that can't match the rounded surfaces, so
  Settings uses our own `Select` (ARIA listbox, keyboard: arrows/Home/End/Enter/Esc/type-ahead).

## 2026-10-17 — Users can use their own whisper.cpp model files
- Any `.bin` file in the model folder that has the GGML magic and isn't a catalog file is listed as
  "Your file · no published checksum" and can be selected. It has no pinned SHA-256 (none exists),
  so it is re-checked for format and for being a plain file inside the model folder before every
  load. Ids are `file:<name>`; names with path separators, drive prefixes or `..` are refused.
- This is the "manual import" route in principles.md (explicit user action, nothing downloaded).
  Catalog files keep the full pinned-hash check.

## 2026-10-17 — CPU speed, CUDA, flash attention (measured)
- Release bench, Whisper Small on CPU: 6 threads RTF 1.27, 8 threads 1.08, 11 threads 1.05 → keep
  the current default (8). whisper.cpp is built with /arch:AVX512 (GGML_NATIVE), so the CPU path
  is already optimized; Large v3 Turbo is simply ~5× real time on this CPU. The app now explains
  this on Models and points to Small/Base or the GPU switch.
- Flash attention (whisper.cpp `flash_attn`): **on for GPU loads only**. Vulkan p50, 12-clip
  corpus: Base 144 → 105 ms, Small 299 → 240 ms, Large v3 Turbo 947 → 724 ms (939 → 729 in the
  A/B run), WER unchanged (2.0%). On the CPU it is slower (Small 5090 → 6544 ms, Turbo 23.3 →
  32.1 s), so CPU loads keep it off.
- CUDA (`gpu-cuda`) cannot be built here: CUDA 12.4 rejects VS 2026 (MSVC 14.51), and with
  `-allow-unsupported-compiler` its front end crashes (`cudafe++` 0xC0000409). It needs a newer
  CUDA 12.x or VS 2022 build tools; CUDA 13 dropped Pascal (GTX 1080). Vulkan already gives
  0.20 RTF for Large v3 Turbo on this GPU. `win-env.cmd` now finds `CUDA_PATH` for when it's possible.
- Shipping risk found: GGML_NATIVE builds for the build machine's CPU (AVX-512 here); a release
  build must use portable CPU variants or it will crash on CPUs without AVX-512. Tracked for the
  release/CPU-build P0 item.

## 2026-10-17 — GPU: Vulkan only, no CUDA or runtime backend picker
- User decision: Vulkan covers every user with a modern GPU (NVIDIA/AMD/Intel) and is fast enough
  (Turbo ~0.15× real time on a GTX 1080). CUDA builds, NVIDIA runtime redistribution, and a
  per-device backend picker are not pursued. The GPU switch + automatic CPU fallback stay as is.

## 2026-10-17 — Keyboard hook self-repair; layout chords only refused when actually used
- Windows silently removes a WH_KEYBOARD_LL hook that misses `LowLevelHooksTimeout` once (no
  notification). The user's recorder showed no keys at all (not even a refusal), so the hook was
  gone. Fix: hook thread runs at THREAD_PRIORITY_TIME_CRITICAL (it only wakes per key), and the
  hook is re-installed every 30 s, after each transcription, and when the recorder opens (new hook
  first, then unhook the old, on the hook thread, so no key is lost or seen twice). Periodic
  injected test keys were rejected as a health check: they would reset the system idle timer.
- Ctrl+Shift / Alt+Shift were always refused as layout-switch chords. Windows only acts on them
  with 2+ keyboard layouts, and `HKCU\Keyboard Layout\Toggle` says which one does what. Now they
  are refused only when this PC uses them (the user's PC: one layout → both allowed). Ctrl+Alt
  stays refused (AltGr). Note for push-to-talk on Ctrl+Shift: Ctrl+Shift+Arrow in editors briefly
  starts and then cancels a recording (existing chord rule).
