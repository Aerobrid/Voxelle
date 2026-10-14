# P0 - Reliable desktop dictation (foundation)
Finish this before anything else.

## 0.1 Global hotkeys and recording
- Configurable global shortcuts that work when the app is unfocused: push-to-talk (where the OS allows key-release detection), toggle, optional cancel.
- Recording indicator (overlay or tray), clear microphone permission errors, device selection, input level meter, recovery from device disconnect.
- Explicit state machine: idle -> recording -> processing -> inserting -> completed | cancelled | failed. Reject illegal transitions. No duplicate sessions. Release the mic on stop, cancel, error, and app exit.
- Document in docs/platform-notes.md: Wayland has no universal global-shortcut API (document portal/compositor options and the fallback); macOS needs Microphone and Accessibility permissions (and Input Monitoring if applicable); Windows blocks input into elevated apps from non-elevated processes.

## 0.2 Local speech recognition
- whisper.cpp unless the repo already has a better working local backend.
- CPU always. GPU acceleration optional, with hardware detection and CPU fallback. Benchmark CPU vs GPU on the dev machine (GTX 1080, 8 GB); do not assume GPU wins.
- Configurable model size and quantization, language selection, optional language detection.
- Model install (with consent, checksum), select, verify, remove; license and source visible.
- Final transcript after recording ends. Partial transcription only if it works reliably. Cancellation and error recovery. A missing or failed model shows a clear error state, never a crash.
- Put speech recognition behind a backend interface (load, transcribe, cancel, unload, capabilities) so engines other than whisper.cpp can be added without touching the rest of the app. whisper.cpp is the first implementation only.
- Pick the default model per machine: detect hardware, run a short benchmark at first setup, and recommend the largest model that meets a latency target (state the target in docs/decisions.md). Users can always override.
- Show each model's measured speed on this machine in the model manager.

## 0.3 Text insertion
Adapter order: native insertion where reliable -> clipboard paste -> manual-copy fallback.
- Save and restore previous clipboard when safe. Do not overwrite a clipboard the user changed during insertion.
- Detect or clearly report cases where insertion cannot work (secure fields, elevated apps, remote desktops, some terminals).
- Result states: inserted | copied-only | failed. Never report "inserted" for copied-only.
- Follow docs/01-security.md section 3.

## 0.4 Settings and history
Language, punctuation, auto-capitalization, paragraph handling, optional filler-word removal (cleanup off by default), history with timestamps, copy/edit/export/delete, retention controls, a switch to disable transcript storage entirely. Store the recognized transcript separately from any AI-edited version.

## Definition of done
From another application: press the global hotkey, speak, stop, and get a correctable transcript inserted at the cursor, entirely offline (verified with networking disabled). Test the full flow, not just units. Report measured: recording start latency, stop-to-transcript latency, real-time factor, error rate on a small documented corpus, insertion success rate, memory/VRAM, startup time, behavior with the model missing.
