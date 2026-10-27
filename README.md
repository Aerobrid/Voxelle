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

## For Users

Pre-built desktop installers (`.exe` / `.msi`) will be available on GitHub Releases once packaged.
1. Download and run the installer.
2. Launch Voxelle from your Start menu.
3. Open **Model Manager** to select or verify a local Whisper speech model.

## Build from Source (Developers)

Requires **Node 20+**, the **Rust toolchain** (`x86_64-pc-windows-msvc`), and **Visual Studio Build Tools with C++ workload** (including MSVC and Ninja).

```bash
# 1. Install frontend dependencies
npm install

# 2. Run unit tests
npm test                                                                  # Frontend tests (Vitest)
scripts\win-env.cmd cargo test --manifest-path src-tauri\Cargo.toml --lib # Rust core tests

# 3. Launch desktop app in development
# On Windows, win-env.cmd auto-detects Ninja, MSVC vcvars64, and Clang paths:
scripts\win-env.cmd npm run tauri dev

# For GPU acceleration (requires LunarG Vulkan SDK):
scripts\win-env.cmd npm run tauri dev -- --features gpu-vulkan

# 4. Build standalone production installer for end users
scripts\win-env.cmd npm run tauri build
```

## License
Apache-2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
