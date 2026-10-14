# Feature status

Status vocabulary (principles.md #8): **implemented** · **experimental** · **platform-limited** ·
**unavailable** · **planned**. Updated with every change. Nothing is marked implemented unless it was
observed working by a command that was run. "experimental" = code + unit tests exist, not yet
observed end-to-end in the running app.

_Last updated: 2026-10-14 (session 2 — audio, ASR (Vulkan), model checks, Windows insertion,
hotkeys, overlay, SQLite storage, UI controls; benchmarks + insertion/mic probes run. Real-voice
E2E from another app not yet run)._

## P0 — Reliable desktop dictation
| Area | Feature | Status | Evidence / notes |
|---|---|---|---|
| 0.1 | State machine idle→recording→processing→inserting→completed/cancelled/failed | implemented | 10 state + 4 session tests pass (`cargo test`), session generation guards stale results |
| 0.1 | Mic capture (cpal), downmix, resample to 16 kHz | implemented | `mic_probe`: real BT headset captured 3.0 s  kHz, mic released after stop; DSP unit-tested. Mic start latency 696 ms on this Bluetooth headset
| 0.1 | Input device selection + level meter | experimental | `list_input_devices`, Settings dropdown, 20 Hz level events; not yet observed |
| 0.1 | Device-disconnect recovery | experimental | cpal error → `disconnected` flag → session Failed + mic released; not exercised |
| 0.1 | Global toggle / push-to-talk / cancel (Windows) | experimental | WH_KEYBOARD_LL hook + pure matcher (11 tests); not yet exercised in the app |
| 0.1 | Recording indicator overlay | experimental | window verified created hidden with WS_EX_NOACTIVATE + WS_EX_TRANSPARENT (click-through) + topmost; showing during a session not yet observed
| 0.1 | Mic released on stop/cancel/error/exit | experimental | stop verified (`mic_probe`: is_capturing=false); cancel/error/exit paths not yet observed
| 0.1 | Clear mic-permission error | experimental | cpal PermissionDenied → message; not exercised |
| 0.2 | whisper.cpp ASR behind `SpeechBackend` | implemented | `tests/asr_model.rs` transcribes corpus clip (WER ≤ 15%), 12-clip bench WER 2.0% |
| 0.2 | CPU inference | implemented | bench: base p50 1.77 s / clip (after the /O2 build fix) |
| 0.2 | GPU (Vulkan) + automatic CPU fallback | implemented (GPU) / experimental (fallback) | GTX 1080 Vulkan 12–27× faster (table below); fallback path written, not forced-tested |
| 0.2 | Cancellation | implemented | test: cancel mid-run → `Cancelled`, backend reusable; granularity = one encoder pass (upstream) |
| 0.2 | Missing / corrupt model → clear error, no crash | implemented | 2 unit tests (missing file, garbage file) |
| 0.2 | Model catalog, pinned SHA-256, GGML magic check, verify, select | implemented (core) | 6 model tests; 4 downloaded models matched pinned hashes |
| 0.2 | In-app model download with consent | planned | Not built yet; Model Manager says so and shows the folder + file name |
| 0.2 | Default model per machine + latency target | experimental | first run auto-selected large-v3-turbo (GPU build) via tested `recommend()`; preload at startup: 2.2 s to ready (36 s first time = one-time hash in debug build). Per-machine first-run benchmark still to do
| 0.2 | Measured speed shown per model | experimental | recorded from real dictations into Model Manager; not yet observed |
| 0.2 | Non-speech hallucination guard | implemented | energy gate (-45 dBFS) tested; whisper alone outputs "you" on noise (test documents it) |
| 0.3 | Insertion ladder: type → paste → copy-only (Windows) | implemented (with known false positive) | `insert-probe.ps1` 19/20 exact (typed ASCII 5/5, Unicode+emoji 5/5, long paste 4/5, multi-line paste 5/5), user clipboard restored. 1 paste reported "inserted" without arriving → add UI Automation verification
| 0.3 | Result states inserted / copied-only / failed | implemented (logic) | never "inserted" for copy paths (tests) |
| 0.3 | Elevated / terminal / remote-desktop / no-focus handling | experimental | detection code + tests for rules; not exercised against real elevated app |
| 0.4 | Settings UI (shortcuts, mic, language, GPU, history on/off, retention) | experimental | editable + validated; persisted in SQLite (tests); not yet clicked through in the app
| 0.4 | History copy / correct / delete / clear / export; raw kept separate from edits | experimental | SQLite store, trigger makes raw_transcript immutable (tested); export to Downloads as JSON; UI not yet clicked through
| 0.4 | Punctuation / capitalization / filler removal | planned | labeled "planned" in Settings
| 0.4 | SQLite persistence (settings, history, model facts, migrations, retention) | implemented | 9 storage tests incl. reopen-from-file, migration refusal of newer schema, SQL-injection text stored verbatim; DB created on real app start
| DoD | End-to-end offline loop from another app, measured | planned | needs app run with mic + hotkey; see manual-tests.md |

### ASR benchmark (2026-10-14, `scripts/bench-asr.ps1`, 12 TTS clips 3.9–5.5 s, en)
Xeon W-2235 (6C/12T, AVX-512, 6 threads) vs GTX 1080 8 GB (Vulkan 1.4.312, driver 582.28).
| Model | CPU p50 | CPU RTF | Vulkan p50 | Vulkan RTF | Vulkan VRAM | Load (GPU) | WER |
|---|---|---|---|---|---|---|---|
| base q5_1 | 1774 ms | 0.378 | 144 ms | 0.030 | +300 MB | 404 ms | 2.0% |
| small q5_1 | 5235 ms | 1.123 | 299 ms | 0.063 | +532 MB | 559 ms | 2.0% |
| medium q5_0 | 15085 ms | 3.251 | 720 ms | 0.153 | +1048 MB | 1280 ms | 2.0% |
| large-v3-turbo q5_0 | 25870 ms | 5.557 | 947 ms | 0.203 | +988 MB | 1168 ms | 2.0% |
WER is identical across models (the same 3 normalization misses: "nine"→"9", "twelve"→"12",
"Our"→"are"): this synthetic corpus is too clean to rank accuracy. A real-voice corpus is a TODO.

## Security controls (01-security.md §7) — status: implemented / tested / not yet
| Control | Status |
|---|---|
| Core ASR runs with no tools/fs/network | implemented (whisper backend has no I/O besides the model file) |
| Renderer context isolation + strict CSP + validated IPC | partial — CSP on both pages, renderer only `core:default`; settings patch rejects unknown keys |
| Transcript rendered as text, never HTML | implemented (React text nodes / textarea only) |
| Insertion strips control chars, never auto-submits | tested (sanitize tests incl. ESC sequences; line breaks never typed) |
| Clipboard save/restore safe | tested (real round-trip, sequence-number guard) |
| Dictated text kept out of Win+V history / cloud clipboard | implemented (format flags), not independently verified |
| Model format allowlist + SHA-256 | tested (GGML magic + pinned hashes) |
| Network egress deny-by-default via single module | not yet used (no network code exists yet) |
| Path canonicalize + allowlist (tools) | not yet (no tools until P4) |
| Policy engine deny-by-default | not yet (P4) |
| Red-team injection fixtures + malicious mock model | not yet (P4) |

## Later priorities (visible as "planned" in UI until their session)
| Priority | Status |
|---|---|
| P1 AI cleanup · P2 translation · P3 vocabulary · P4 assistant · P5 MCP · P6 notes/search · P7 audio import · P8 meetings · P9 perf | planned |

## Project foundation
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
| Rust core compiles + tests pass | implemented | 2026-10-14: `cargo test` 63 unit + 4 model tests pass (Vulkan build)

