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
