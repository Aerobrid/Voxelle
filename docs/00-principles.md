# Principles, architecture, and workflow

## Product
Desktop app (Windows, Linux, macOS): Wispr Flow-style dictation plus local AI cleanup, translation, a vocabulary/speech coach, a voice assistant, an MCP server, notes, audio import, and meeting transcription. Free, no account, offline-capable.

## Requirements (apply to every feature)
1. Fully local and offline-capable by default.
2. No subscription, API key, mandatory account, backend, cloud database, telemetry, or external AI API.
3. Audio, transcripts, vocabulary profiles, and notes stay on the user's machine by default.
4. Optional external integrations: explicit opt-in, off by default, disclose what data leaves the device, show a local/remote indicator on every operation that could use a remote service.
5. Granular settings for models, privacy, shortcuts, language, transcription, AI features, resources. Every visible setting is wired to working behavior or labeled "planned".
6. Modular: new models, tools, and integrations can be added without rewriting the core.
7. Permissive or compatible licenses only. Verify before bundling or distributing any model or dataset. Record in docs/dependencies.md.
8. Never fabricate functionality. Status vocabulary: implemented / experimental / platform-limited / unavailable / planned.
9. Core dictation never depends at runtime on REA, Claude Code, Ollama, or any external service.

## Offline mode
- One global switch blocks all optional network requests, enforced in a single network module.
- Models arrive only through the model manager after explicit consent, or by manual import. No silent downloads. Confirm before any multi-GB download.
- No crash reports, telemetry, audio, transcripts, or screenshots leave the device without explicit consent.
- Test offline behavior with networking actually disabled where possible.

## Architecture rules
- Reuse working architecture. No rewrite without a concrete technical reason logged in docs/decisions.md.
- Keep components separable: UI/settings; audio capture; speech recognition and model management; dictation post-processing; global shortcuts and platform integration; text insertion and clipboard; translation; LLM provider layer; vocabulary and speech analysis; meetings; notes and search; tool execution and policy engine; MCP; storage, privacy, diagnostics.
- Inference and audio work run off the UI thread in workers or processes with explicit lifecycle: start, health check, cancel, stop, cleanup on exit and crash.
- SQLite for settings, history, notes, vocabulary, metadata. Use migrations and parameterized queries. Provide export and backup. Model weights never go in the database; they live in a user-selectable model directory.

## Model manager
Show per model: name, purpose, local/remote, disk size, memory needs if known, languages, hardware compatibility, license and source, install status, integrity check. Actions: install/import, verify (SHA-256), select, unload, remove, retry.
Separate model selection for: speech recognition, cleanup/rewrite, vocabulary, embeddings, translation, diarization.
Never load every model at once. Load on demand, unload on idle or user action, configurable limits. The GTX 1080 has 8 GB VRAM: assume a model may not fit, keep a CPU fallback, and benchmark before choosing defaults.

## UI
Keep it consistent with the existing project. Sections: dashboard, dictation history, vocabulary coach, AI assistant, notes, audio import, model manager, settings, privacy dashboard.
Privacy dashboard shows: which features are local, which can use the network, whether recording is active, whether a transcript is being retained, installed models and locations, the data directory, export and deletion controls.
Settings: hotkeys, microphone, language, default model, model directory, CPU/GPU, AI cleanup, translation, vocabulary goals, retention, local-only mode, MCP permissions, theme and accessibility.

## Audit (when a repo exists)
Identify: desktop framework, UI framework, language, audio capture, transcription engine, hotkey mechanism, text insertion, model integrations, storage, test setup, dependencies and licenses. Run the app and tests. Classify each feature as complete / partial / stubbed / broken. Write docs/architecture.md, docs/feature-status.md, docs/implementation-roadmap.md. Do not stop after documenting; start the highest-priority incomplete feature.

## Default stack (only if no repo exists)
Tauri 2 + Rust core + React/TypeScript UI; whisper.cpp (sidecar or bindings); SQLite; cpal for audio capture. Log the rationale in docs/decisions.md. If a repo exists, keep its stack.

## Testing and measurement
Automated tests: recording state transitions, cancellation and failure recovery, transcription pipeline, insertion fallbacks, clipboard preservation, AI cleanup preserving the original transcript, vocabulary ranking and known-word exclusion, migrations, export and deletion, MCP permissions and validation, offline-mode enforcement, model download verification, platform adapters where practical.
Manual test procedures in docs/manual-tests.md for Windows, Linux X11, Linux Wayland (where supported), macOS.
Measure: recording start latency, stop-to-transcript latency, real-time factor, error rate on a small documented corpus, insertion success rate, memory and VRAM, startup time, missing/failed-model behavior. Use repeatable benchmarks. Do not claim improvements the measurements do not show.

## Reliability
Crash recovery, proper process cleanup, mic/recording state management, failures recoverable without restart, data export and deletion, dependency and license review, tests for privacy-sensitive paths.
