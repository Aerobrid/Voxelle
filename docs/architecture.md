# Architecture — Voxelle

Local-first, offline-by-default dictation + voice-AI desktop app. This document is the map; module
behavior is authoritative in code and in `feature-status.md`.

## Stack
- **Shell:** Tauri 2 (Rust host process + WebView2/WebKit renderer). The renderer has **no Node/OS
  access** (context isolation, strict CSP, validated IPC commands only) — security.md §6.
- **Core:** Rust, split into modules with explicit lifecycles (start/health/cancel/stop/cleanup).
- **UI:** React + TypeScript + Vite. Talks to the core only through typed Tauri IPC commands + events.
- **ASR:** whisper.cpp via `whisper-rs` (sidecar binary fallback). Behind a `SpeechBackend` trait.
- **Audio:** `cpal` capture on a dedicated thread.
- **Storage:** SQLite (migrations, parameterized queries). Weights live outside the DB.

## Process & thread model
```
┌────────────────────────────────────────────────────────────────┐
│ Tauri core process (Rust)                                        │
│                                                                  │
│  UI (WebView) ──IPC commands──►  Command layer (validated)       │
│       ▲                               │                          │
│       └──────events (state,level)─────┤                          │
│                                       ▼                          │
│   ┌─────────────┐   ┌──────────────┐   ┌────────────────────┐    │
│   │ Hotkey      │   │ Session /    │   │ Storage (SQLite)   │    │
│   │ adapter     │──►│ State machine│──►│ settings/history   │    │
│   └─────────────┘   └──────┬───────┘   └────────────────────┘    │
│                            │                                     │
│        ┌───────────────────┼───────────────────┐                │
│        ▼                   ▼                   ▼                 │
│  ┌───────────┐      ┌──────────────┐    ┌──────────────┐         │
│  │ Audio     │ PCM  │ Speech       │    │ Text         │         │
│  │ capture   │─────►│ backend      │───►│ inserter     │         │
│  │ (cpal/thr)│      │ (whisper/thr)│    │ (adapter)    │         │
│  └───────────┘      └──────────────┘    └──────────────┘         │
│                                                                  │
│  Network module (single egress point; offline switch) — unused   │
│  by the P0 core loop; gates model downloads only.                │
└────────────────────────────────────────────────────────────────┘
```
Audio capture and inference run **off the UI thread** (dedicated threads/workers) with explicit
cancel and cleanup on stop/error/exit. No inference or audio work blocks IPC.

## Module boundaries (separable per principles.md)
- `ui/` — React app: UI, settings screens, state mirror. No business logic beyond presentation.
- `core/session` — the dictation **state machine** (below) and session lifecycle. Single source of
  truth; rejects illegal transitions; guarantees no duplicate sessions and mic release.
- `core/audio` — `AudioCapture` trait + `cpal` implementation: device enumeration, selection, level
  metering, start/stop, disconnect recovery.
- `core/asr` — `SpeechBackend` trait (`load / transcribe / cancel / unload / capabilities`) +
  `whisper-rs` implementation. Engine-agnostic so other backends can be added without touching the
  rest. CPU always; optional GPU with CPU fallback.
- `core/insert` — `TextInserter` trait + per-OS adapters. Order: native insert → clipboard paste →
  manual-copy fallback. Strips control chars; never auto-submits (security.md §3). Clipboard
  save/restore when safe. States: inserted / copied-only / failed.
- `core/hotkey` — `HotkeyBackend` trait + per-OS adapters. Toggle/cancel via Tauri global-shortcut;
  push-to-talk key-release via a key-state layer.
- `core/models` — model manager: list/install/import/verify(SHA-256)/select/unload/remove, per-model
  metadata (purpose, local/remote, size, languages, hardware, license, source, measured speed).
- `core/storage` — SQLite: settings, dictation history (raw transcript stored separately from any
  AI-edited text), retention, export/delete, migrations.
- `core/net` — the **single** network module. Deny-by-default; enforces offline mode. Only the model
  manager uses it (consented downloads). The P0 dictation loop never calls it.
- `core/policy` *(P4+)* — central deny-by-default policy engine for tools/MCP (security.md §2). Not in
  P0; cleanup/translation/vocabulary LLM calls run with **no tools/fs/network**.

## Dictation state machine (P0 §0.1)
```
            start (hotkey/UI)                stop                 transcript ready
  idle ───────────────────► recording ──────────────► processing ───────────────► inserting
   ▲                           │  │                       │                           │
   │        cancel             │  │ device error          │ asr error                 │ ok
   ├───────────────────────────┘  └──────────┐            ▼                           ▼
   │                                          ▼         failed ◄───────────────── (insert error)
   │                                       failed                                     │
   │                                          │                                   completed
   └──────────────── (reset) ◄────────────────┴──────────────── (reset) ◄─────── copied-only
```
States: `idle · recording · processing · inserting · completed · cancelled · failed`. Illegal
transitions are rejected. Entering any terminal/cancel/error state **releases the mic**. Exactly one
active session at a time (new start requests while non-idle are refused, not queued).

## Security posture (summary; authoritative in 01-security.md)
- Transcripts and all imported/retrieved text are **untrusted data**, never instructions.
- Core ASR/cleanup path: **no tools, no fs, no network.** Tools only exist in the assistant/MCP
  features behind the policy engine (later priorities).
- Renderer: context isolation + strict CSP + validated IPC; transcripts rendered as text, never HTML.
- Paths canonicalized + allowlisted at open time; tools can never touch DB/config/manifests/secrets.
- Network egress only through `core/net`; offline mode enforced there.

## Data & storage
- SQLite file in a user-only data dir. Tables (P0): `settings`, `dictation_history`
  (raw transcript column distinct from any `edited_text`), `models` (metadata + measured speed),
  `migrations`. Export (JSON/text) + delete + "disable transcript storage entirely" switch.
- Model weights live in a user-selectable model directory, never in the DB, never committed.

## What P0 builds vs. defers
- **P0 builds:** the full loop — hotkey → record → whisper → insert — offline, plus model manager
  (local ASR), settings, history, privacy dashboard essentials, and the state machine + adapters.
- **Deferred:** AI cleanup (P1), translation (P2), vocabulary coach (P3), assistant + policy engine +
  MCP (P4/P5), notes/search (P6), audio import (P7), meetings (P8), perf (P9). These sections appear
  in the UI as **planned** until their priority lands.
