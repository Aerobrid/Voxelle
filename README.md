# Voxelle

Local, private-first dictation and voice AI for the desktop. Voxelle turns your speech into text
and inserts it at the cursor in any application — **fully offline by default**. No account, no
subscription, no cloud transcription, no telemetry. Your audio and transcripts stay on your machine.

> Status: **early development.** The foundation and app shell are in place; the end-to-end dictation
> loop is being built under priority P0. Features that are not implemented yet are clearly labeled
> *planned* in the UI — nothing is faked.

## Principles
- **Local & offline by default.** The core dictation loop never touches the network. Any optional
  network use is opt-in, off by default, and routed through a single, auditable module.
- **Your data is yours.** Transcripts and audio are stored locally (or not at all — you choose).
  The raw transcript is always kept separate from any AI-edited version.
- **No fabricated functionality.** Every visible control either works or is labeled planned.

## Platforms
Windows, Linux, and macOS. Primary development is on Windows.

## Tech
- Desktop shell: **Tauri 2** (Rust core + web UI; the UI has no direct OS access).
- UI: **React + TypeScript + Vite**.
- Speech recognition: **whisper.cpp**, local, behind a pluggable engine interface (CPU always; GPU
  optional with a CPU fallback).
- Audio: **cpal**. Storage: **SQLite**.

## Build (development)
Requires Node 20+, the Rust toolchain, and the C++ build tools for your platform
(on Windows: Visual Studio Build Tools with the C++ workload).

```bash
npm install
npm run build      # type-check + build the web UI
npm test           # run UI unit tests
npm run tauri dev  # run the desktop app (requires the Rust toolchain + C++ build tools)
```

## License
Apache-2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
