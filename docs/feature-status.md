# Feature status

Status vocabulary (principles.md #8): **implemented** · **experimental** · **platform-limited** ·
**unavailable** · **planned**. Updated with every change. Nothing is marked implemented unless it was
observed working by a command that was run. "experimental" = code + unit tests exist, not yet
observed end-to-end in the running app.

_Last updated: 2026-10-17 (session 5 — hook self-repair, Ctrl+Shift when unused, Stats double count; session 4 — shortcut rules + recorder dialog, startup repair of bad
saved shortcuts, toasts, custom dropdown, bundled Inter/JetBrains Mono, Stats page, your-own-model
support, GPU flash attention (−22% on Turbo), CPU advice, CUDA build attempt (blocked by toolchain))._

## P0 — Reliable desktop dictation
| Area | Feature | Status | Evidence / notes |
|---|---|---|---|
| 0.1 | State machine idle→recording→processing→inserting→completed/cancelled/failed | implemented | 10 state + 4 session tests pass (`cargo test`), session generation guards stale results |
| 0.1 | Mic capture (cpal), downmix, resample to 16 kHz | implemented | 2026-10-15: one process-lifetime MTA audio thread; "System default" resolved to its concrete endpoint. `capture_repeat` 20/20 rounds + real app 6/6 start/stop (was failing from round 3 with RPC_E_CHANGED_MODE); full-length audio (was missing ~0.6 s at start). One real-voice user dictation succeeded before the fix. DSP unit-tested
| 0.1 | Keep microphone ready (optional, default off) | implemented | `capture_repeat --keep`: next start 0 ms, closes after the idle timeout, cancel releases the mic. Settings dropdown not yet clicked through
| 0.1 | "Starting mic…" until first audio + slow-mic hint | experimental | `first_audio_ms` measured per recording (probe); overlay label unit-tested; not yet seen during a real Bluetooth start-up
| 0.1 | Input device selection + level meter | experimental | `list_input_devices`, Settings dropdown, 20 Hz level events; not yet observed |
| 0.1 | Device-disconnect recovery | experimental | cpal error → `disconnected` flag → session Failed + mic released; not exercised |
| 0.1 | Stats: each dictation counted once | implemented (fix) | Was counted twice: StrictMode double-mount left a stale result listener (async subscribe). Fixed in App.tsx; tsc + 24 tests pass |
| 0.1 | Global toggle / push-to-talk / cancel (Windows) | experimental | WH_KEYBOARD_LL hook + pure matcher (16 tests incl. Ctrl+Win chords + Start-menu mask). User's toggle hotkey started/stopped a real dictation once (2026-10-15); PTT/cancel/chords not yet exercised by a person |
| 0.1 | Shortcut recorder (press keys, incl. Win) | implemented (core) / experimental (physical keys in the dialog) | `hotkeys/capture.rs` 12 tests (role rules, Esc backs out, only owned key-ups swallowed). Real app 2026-10-17: dialog opens, 15 s idle → "Stopped listening" (no error); refusal, save-conflict and saved+toast states rendered from injected recorder events. A person pressing Ctrl+Win / a letter / Win in the dialog: not yet (manual-tests step 13) |
| 0.1 | Keyboard hook self-repair (re-installed every 30 s, after each transcription, and when the recorder opens; hook thread at time-critical priority) | experimental | Windows silently drops an LL hook that misses its timeout (likely during the user's 51 s CPU transcription → recorder saw no keys). 101 unit tests pass; app rebuilt. Not yet confirmed by a person pressing keys (manual-tests step 13) |
| 0.1 | Shortcut rules (no plain/typing keys, no Win alone, no AltGr, no layout chords that this PC uses (Ctrl+Shift / Alt+Shift allowed with one layout), no Windows shortcuts, no shared keys) | implemented | 9 new tests; real app IPC refused A, LeftWin, Ctrl+Alt, a duplicate, Win+Shift+S with readable reasons |
| 0.1 | Startup repair of saved shortcuts the rules refuse | implemented | 2 tests; real app reset the user's LeftWin push-to-talk + Ctrl+Space hands-free to defaults (logged, persisted) |
| 0.1 | Tray icon, close-to-tray, single instance | implemented (window/instance) / experimental (tray menu) | 2026-10-15 real app: WM_CLOSE → window hidden, process alive; launching a 2nd instance → 1 process, window shown. Tray created without error; menu items not yet clicked |
| 0.1 | Recording indicator overlay | experimental | window verified created hidden with WS_EX_NOACTIVATE + WS_EX_TRANSPARENT (click-through) + topmost; showing during a session not yet observed
| 0.1 | Mic released on stop/cancel/error/exit | experimental | stop verified (`mic_probe`: is_capturing=false); cancel/error/exit paths not yet observed
| 0.1 | Clear mic-permission error | experimental | cpal PermissionDenied → message; not exercised |
| 0.2 | whisper.cpp ASR behind `SpeechBackend` | implemented | `tests/asr_model.rs` transcribes corpus clip (WER ≤ 15%), 12-clip bench WER 2.0% |
| 0.2 | CPU inference | implemented | bench: base p50 1.77 s / clip (after the /O2 build fix) |
| 0.2 | GPU (Vulkan) + automatic CPU fallback | implemented (GPU) / experimental (fallback) | GTX 1080 Vulkan 12–27× faster (table below); fallback path written, not forced-tested |
| 0.2 | Flash attention on GPU loads | implemented | Release bench 2026-10-17 Vulkan p50: Base 105 ms, Small 240 ms, Turbo 724 ms (was 144/299/947), WER unchanged; CPU keeps it off (measured slower) |
| 0.2 | CUDA build (`gpu-cuda`) | unavailable (toolchain) | CUDA 12.4 rejects VS 2026; with the override its front end crashes. Needs newer CUDA 12.x or VS 2022 tools; CUDA 13 dropped the GTX 1080 |
| 0.2 | CPU advice when a heavy model runs on the CPU | implemented | 3 unit tests; real app showed it with the measured 8.1× (GPU off, Turbo) |
| 0.2 | Your own whisper.cpp model files (folder scan, select, format check each load, no checksum) | implemented | 2 tests incl. path-traversal ids; real app: listed, selected, loaded on GPU. "Open folder" button not clicked (would open Explorer on the user's desktop) |
| 0.2 | Cancellation | implemented | test: cancel mid-run → `Cancelled`, backend reusable; granularity = one encoder pass (upstream) |
| 0.2 | Missing / corrupt model → clear error, no crash | implemented | 2 unit tests (missing file, garbage file) |
| 0.2 | Model catalog, pinned SHA-256, GGML magic check, verify, select | implemented (core) | 6 model tests; 4 downloaded models matched pinned hashes |
| 0.2 | In-app model download with consent | planned | Not built yet; Model Manager says so and shows the folder + file name |
| 0.2 | Default model per machine + latency target | experimental | first run auto-selected large-v3-turbo (GPU build) via tested `recommend()`; preload at startup: 2.2 s to ready (36 s first time = one-time hash in debug build). Per-machine first-run benchmark still to do
| 0.2 | Measured speed shown per model | experimental | recorded from real dictations into Model Manager; not yet observed |
| 0.2 | Non-speech hallucination guard | implemented | energy gate (-45 dBFS) tested; whisper alone outputs "you" on noise (test documents it) |
| 0.3 | Insertion ladder: type → paste → copy-only (Windows) | implemented (with known false positive) | `insert-probe.ps1` 19/20 exact (typed ASCII 5/5, Unicode+emoji 5/5, long paste 4/5, multi-line paste 5/5), user clipboard restored. 1 paste reported "inserted" without arriving → add UI Automation verification
| 0.3 | Result states inserted / copied-only / failed | implemented (logic) | never "inserted" for copy paths (tests) |
| 0.3 | Copy buttons / copy-only fallback put text on the clipboard | implemented | 2026-10-15: real app `copy_text` IPC → read back by another process. Deliberate copies are normal Win+V history entries, verified by read-back before reporting success (2 real-clipboard tests). History Copy button not clicked yet (history was empty)
| 0.3 | Elevated / terminal / remote-desktop / no-focus handling | experimental | detection code + tests for rules; not exercised against real elevated app |
| 0.4 | Settings UI (shortcuts, mic, language, GPU, history on/off, retention) | experimental | editable + validated; persisted in SQLite (tests); not yet clicked through in the app
| 0.4 | History copy / correct / delete / clear / export; raw kept separate from edits | experimental | SQLite store, trigger makes raw_transcript immutable (tested); export to Downloads as JSON. 2026-10-15: user's real dictations appear in History (4 entries); grouped by day + search (helpers unit-tested); copy/delete buttons not yet clicked by a person
| UI | v2 design: frameless window + own title bar, sidebar, voice orb, grouped settings, switches | implemented | ui-direction.md v2. Real app screenshots reviewed for Home (idle + recording), History, Settings, Models, Privacy. Title bar: maximize/restore and close→tray verified via the real buttons; minimize not exercised. Shortcuts saved in canonical form (test) |
| UI | Toasts (copy, save, shortcut confirmations), custom rounded dropdown, recorder dialog, bundled Inter + JetBrains Mono | implemented | Real app screenshots 2026-10-17: Home, Settings (aligned key fields, open dropdown flips up near the edge), dialog states, toast after a shortcut save. Select type-ahead unit-tested. History Copy toast not clicked (would overwrite the user's clipboard) |
| UI | Stats page (this session's timings, typical stop→text/speed, per-model measured speed); slow-mic note moved here from Home | implemented (empty + model table) / experimental (per-dictation rows) | Real app: empty state + model table with the measured Turbo CPU speed. Rows appear after a dictation (not run: would record the user's mic) |
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

GPU with flash attention (2026-10-17, release bench, same corpus): Base 105 ms (RTF 0.022), Small
240 ms (0.050), Turbo 724 ms (0.154); WER 2.0%. CPU threads (Small): 6 → RTF 1.27, 8 → 1.08, 11 → 1.05.
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
| Model format allowlist + SHA-256 | tested (GGML magic + pinned hashes for catalog models). Files the user adds: GGML magic + plain-file-in-folder check before every load, no hash exists — labeled "no published checksum" in the UI (decisions 2026-10-17) |
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

