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
