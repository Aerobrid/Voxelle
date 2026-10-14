# Feature status

Status vocabulary (principles.md #8): **implemented** · **experimental** · **platform-limited** ·
**unavailable** · **planned**. Updated with every change. Nothing is marked implemented unless it was
observed working by a command that was run.

_Last updated: 2026-10-13 (session 1 — foundation + scaffold. Frontend builds + tests pass; Rust
core written & parses but not yet compiled, MSVC Build Tools pending)._

## P0 — Reliable desktop dictation
| Area | Feature | Status | Notes |
|---|---|---|---|
| 0.1 | Global toggle hotkey | planned | Design in architecture/platform-notes; code in next slice |
| 0.1 | Push-to-talk (key-release) | planned | Needs key-state layer (not Tauri global-shortcut alone) |
| 0.1 | Cancel hotkey | planned | |
| 0.1 | Recording indicator (tray/overlay) | planned | Waveform motif per ui-direction |
| 0.1 | Mic device selection + level meter | planned | via cpal |
| 0.1 | Device-disconnect recovery | planned | |
| 0.1 | State machine (idle→…→completed) | experimental | Implemented in `session/state.rs` + `mod.rs` with 10 unit tests; **not yet run** (needs toolchain). UI mirror `transitions.ts` tested (5 tests pass) |
| 0.2 | whisper.cpp local ASR | planned | Behind `SpeechBackend` trait; blocked on toolchain |
| 0.2 | CPU inference | planned | |
| 0.2 | GPU accel + CPU fallback | planned | GTX 1080 detected; benchmark before defaulting |
| 0.2 | Model install/verify(SHA-256)/select/remove | planned | Consent + checksum required |
| 0.2 | Model speed shown per machine | planned | |
| 0.3 | Native/clipboard/manual insertion ladder | planned | Clipboard save/restore; no auto-Enter |
| 0.3 | Result states inserted/copied-only/failed | planned | Never report "inserted" for copied-only |
| 0.4 | Settings (lang, punctuation, retention, …) | planned | |
| 0.4 | History (timestamps, copy/edit/export/delete) | planned | Raw transcript stored separately from edited |
| 0.4 | Disable-transcript-storage switch | planned | |
| DoD | End-to-end offline loop, measured | planned | **Blocked** on MSVC Build Tools + a whisper model |

## Later priorities (visible as "planned" in UI until their session)
| Priority | Status |
|---|---|
| P1 AI cleanup · P2 translation · P3 vocabulary · P4 assistant · P5 MCP · P6 notes/search · P7 audio import · P8 meetings · P9 perf | planned |

## Project foundation (this session)
| Item | Status | Evidence |
|---|---|---|
| Repo audit (empty → default stack) | implemented | decisions.md; `git log` empty before scaffold |
| Docs: architecture / roadmap / feature-status | implemented | this file + siblings created |
| UI direction + palette from logo | implemented | ui-direction.md |
| .gitignore from template | implemented | `.gitignore` created; verify no ignored file staged before commit |
| Branding organized (master SVG) | implemented | `assets/branding/voxelle-logo-master.svg` |
| Reference survey (Handy/OpenWhispr + 2) | implemented | docs/reference-survey.md written (4 projects, licenses read) |
| Tauri + React/TS scaffold | implemented | hand-rolled Tauri 2 + React/Vite; builds |
| Frontend builds (Node only) | implemented | `npm run build` → tsc + vite OK (62 modules); `npm test` → 5/5 pass |
| App icons derived from logo | implemented | `scripts/make-icon.mjs` → `tauri icon` full set in `src-tauri/icons/` |
| Rust core authored + parses | experimental | state machine, traits, commands, main written; `rustfmt` parses all; **not compiled** |
| Rust core compiles + tests pass | blocked | needs MSVC Build Tools (then `cargo test`) |

## Security controls (01-security.md §7) — status: implemented / tested / not yet
| Control | Status |
|---|---|
| Core ASR/cleanup runs with no tools/fs/network | not yet (enforced by architecture; assert in tests) |
| Renderer context isolation + strict CSP + validated IPC | not yet |
| Transcript rendered as text, never HTML | not yet |
| Insertion strips control chars, never auto-submits | not yet |
| Clipboard save/restore safe | not yet |
| Path canonicalize + allowlist (tools) | not yet (no tools until P4) |
| Network egress deny-by-default via single module | not yet |
| Model format allowlist (GGUF/safetensors/ONNX) + SHA-256 | not yet |
| Policy engine deny-by-default | not yet (P4) |
| Red-team injection fixtures + malicious mock model | not yet (P4; design noted) |
