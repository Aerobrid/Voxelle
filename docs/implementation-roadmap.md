# Implementation roadmap

Ordered by the priority files. Each priority ends only when its definition of done is verified by a
command that was run (CLAUDE.md session protocol). This session covers foundation + the start of P0.

## Now — Foundation (session 1)
1. ✅ Audit (empty repo → default stack) + decisions.md.
2. ✅ Core docs: architecture, ui-direction, dependencies, platform-notes, feature-status, roadmap.
3. ✅ `.gitignore`, branding organized.
4. ⏳ Reference survey (subagent).
5. ⏳ Scaffold Tauri 2 + React/TS; frontend builds on Node.
6. ⛔ Rust build + whisper — **blocked on MSVC Build Tools** (user installing).

## P0 — Reliable desktop dictation (current priority)
Smallest-useful-slice order:
1. **Scaffold & shell** — Tauri 2 app window + React shell (nav rail, status strip, placeholder
   sections labeled planned). Frontend builds. *(Rust compiles once toolchain lands.)*
2. **State machine** — `core/session`: states + transitions + guards. Pure Rust, unit-tested (runs
   without audio/ASR). This is the backbone and testable first.
3. **Audio capture** — `core/audio` cpal: enumerate devices, select, start/stop, level meter,
   disconnect recovery. Manual + unit tests for state transitions.
4. **Speech backend** — `core/asr` trait + whisper-rs impl. Model load/transcribe/cancel/unload.
   Missing/failed model = clear error, never crash. Benchmark CPU vs GPU on GTX 1080; set default +
   latency target in decisions.md.
5. **Model manager** — `core/models`: install (consent+SHA-256)/import/verify/select/remove; show
   measured speed. Weights in a user dir; the single network module handles downloads.
6. **Text insertion** — `core/insert` ladder (native→clipboard→manual), clipboard restore, result
   states, control-char stripping, no auto-Enter. Windows adapter first.
7. **Hotkeys** — toggle + cancel (global-shortcut) and push-to-talk (key-state layer). Windows first;
   document Wayland/macOS limits.
8. **Settings & history** — SQLite storage, settings screen, history (copy/edit/export/delete,
   retention, disable-storage switch). Raw vs edited stored separately.
9. **End-to-end + measurement** — full offline loop from another app; measure start latency,
   stop→transcript, RTF, error rate on a small corpus, insertion success, memory/VRAM, startup,
   model-missing behavior. Verify with networking disabled. → P0 done.

Platforms: Windows adapters first (dev machine); Linux/macOS adapters behind the same traits,
marked platform-limited where an OS can't honor a capability.

## After P0 (do not start until P0 done)
- P1 AI cleanup (local LLM, no tools, raw transcript preserved) → P2 translation → P3 vocabulary coach
  → P4 voice assistant **+ policy engine + red-team harness (security.md §7)** → P5 MCP → P6
  notes/search → P7 audio import → P8 meetings → P9 performance.

## Cross-cutting, maintained every session
- Update `feature-status.md` + `progress.md` each slice. Tests for every privacy-sensitive path.
- New dependency → justification + license in `dependencies.md`. Commit after each completed slice.
