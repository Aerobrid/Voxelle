# Reference survey: open-source dictation / push-to-talk STT projects

## Purpose and policy

This document surveys open-source local speech-to-text / dictation projects so Voxelle can
learn **cross-platform techniques** for global hotkeys, push-to-talk key-release detection,
audio capture, text insertion, and local ASR. We write **original code**.

License gate for *copying* code (per project policy in `CLAUDE.md`):
- Copying code is allowed **only** from projects licensed **MIT, Apache-2.0, or BSD**, and only
  after telling the user which file + license, keeping the license notice, adding attribution to
  `NOTICE`, and recording it in `docs/dependencies.md`.
- **Do not copy** from GPL / AGPL / unlicensed projects. We may read them to *understand* an
  approach and reimplement it in our own code, but no code, strings, or assets may be lifted.

Findings here are factual (read from each repo's source / LICENSE / README). Where something
was not confirmed in source it is marked "unconfirmed".

Repos were shallow-cloned into `C:\Users\Raj Patel\Downloads\references\` (outside the Voxelle
repo, never committed): `handy/`, `openwhispr/`, `whispering/`, `whisperwriter/`.

Quick license summary:

| Project       | License          | Copyable under our policy? |
|---------------|------------------|----------------------------|
| Handy         | **MIT**          | Yes (with attribution)     |
| OpenWhispr    | **MIT**          | Yes (with attribution)     |
| Whispering    | **AGPL-3.0-or-later** | No (learn only)       |
| WhisperWriter | **GPL-3.0**      | No (learn only)            |

---

## 1. Handy

- **Repo:** https://github.com/cjpais/Handy
- **License:** **MIT** (`LICENSE`: "MIT License, Copyright (c) 2025 CJ Pais"). Copyable under our policy with attribution.
- **Tech stack:** Tauri 2 (Rust backend `src-tauri/`, React + TypeScript + Tailwind v4 frontend).
  This is the closest architectural match to Voxelle (Tauri 2 + Rust + React/TS).
- **Hotkey / global shortcut (incl. key-release for push-to-talk):**
  - Uses a dedicated crate **`handy-keys`** (its own lower-level hotkey library) rather than only
    Tauri's `global-shortcut` plugin, "for more control over keyboard events." A dedicated
    **manager thread owns the `HotkeyManager`**, polls for events, and dispatches actions;
    register/unregister come from the main thread over an mpsc channel. This single-threaded
    owner pattern keeps the hotkey state thread-safe.
    (`src-tauri/src/shortcut/handy_keys.rs`)
  - It also lists `rdev` (rustdesk fork) and Tauri's `tauri-plugin-global-shortcut` in
    `Cargo.toml`. `handy-keys` exposes `Hotkey`, `HotkeyState`, and a `KeyboardListener`, giving
    explicit key-down vs key-up state — this is what enables **hold-to-record** (record on press,
    stop and transcribe on release) alongside tap-to-toggle and hold-only modes (README).
- **Audio capture:** Native via **cpal** (through `rodio::cpal` and a `handy-recorder` crate).
  Device enumeration in `src-tauri/src/audio_toolkit/audio/device.rs`. VAD to trim silence uses
  **Silero** (ONNX) plus the **`earshot`** crate (`src-tauri/src/audio_toolkit/vad/`).
- **Text insertion (per platform):** Clipboard + synthetic paste keystroke, via **`enigo`**.
  - It saves the current clipboard, writes the transcript, sends paste, then restores the clipboard
    (`src-tauri/src/clipboard.rs`).
  - Paste is sent as **Ctrl+V / Cmd+V using platform virtual key codes** so it works regardless of
    keyboard layout (Russian / AZERTY / Dvorak). On macOS it resolves the physical "V" key via
    Carbon `UCKeyTranslate` / TIS APIs on the main thread (`src-tauri/src/input.rs`).
  - **Linux is handled explicitly:** on Wayland it prefers `wl-copy` for clipboard and `wtype`
    for the key combo (but skips `wtype` on KDE/GNOME because of missing
    `zwp_virtual_keyboard_manager_v1`), then falls back to `dotool` / `ydotool`; on X11 it prefers
    `xdotool` then `ydotool`. It can also directly type text via `wtype`/`kwtype`/`dotool`.
    A user setting (`TypingTool`) lets users force a tool.
- **Transcription engine:** Local Whisper via **`transcribe-cpp`** (a whisper.cpp binding) and
  **`transcribe-rs`** (with the `onnx` feature) for Parakeet V3. **GPU:** on Windows x86_64 it
  ships the **Vulkan** whisper backend (ideal for our GTX 1080), **Metal** on macOS; CPU fallback
  with runtime CPU dispatch; Windows-on-ARM and the ONNX path are CPU-only (`Cargo.toml` comments).
- **Platform notes / known issues:** README states fully offline, Windows/macOS/Linux. The
  extensive Wayland/X11 tool-detection branching in `clipboard.rs` is itself the evidence that
  Linux text injection is the hardest surface (KDE vs GNOME vs wlroots differ). Homebrew/winget
  packages are explicitly not maintained by the authors.
- **Lessons for Voxelle:**
  1. Own the hotkey loop on a dedicated thread with an explicit key-down/key-up state machine so
     push-to-talk "record on press, transcribe on release" is deterministic — do not rely solely on
     Tauri's global-shortcut plugin, which is press-oriented.
  2. For Linux insertion, detect the session (Wayland vs X11, and compositor: KDE/GNOME/wlroots)
     and pick the injection tool at runtime (`wtype`/`ydotool`/`dotool` vs `xdotool`), with a
     user override — a single method will not cover Linux.

---

## 2. OpenWhispr

- **Repo:** https://github.com/OpenWhispr/openwhispr (package name `open-whispr`)
- **License:** **MIT** (`LICENSE`: "MIT License, Copyright (c) 2024 OpenWhispr Team"). Copyable
  under our policy with attribution.
- **Tech stack:** **Electron** (`main.js`, `preload.js`) + React + Vite + Tailwind frontend,
  with a set of **compiled native per-platform helper binaries** (C, Objective-C/Swift).
- **Hotkey / global shortcut (incl. key-release for push-to-talk):** Does **not** rely on
  Electron's `globalShortcut` for push-to-talk; instead it spawns **small native "key listener"
  helper processes** that emit `KEY_DOWN` / `KEY_UP` lines on stdout, which the Electron main
  process reads:
  - **Windows:** `resources/windows-key-listener.c` — a **low-level keyboard hook** (`WH_KEYBOARD_LL`,
    `KBDLLHOOKSTRUCT`, `WM_KEYUP`/`WM_SYSKEYUP`). Takes a virtual key code + modifier requirements
    as args; tracks modifier state; emits KEY_DOWN/KEY_UP. (Note in source: `GetAsyncKeyState` is
    unreliable for the key that triggered the current hook callback, so it tracks state in the hook.)
  - **Linux:** `resources/linux-key-listener.c` — reads the **evdev** subsystem (`/dev/input`,
    `linux/input.h`, epoll + inotify for hotplug) across all keyboards; same KEY_DOWN/KEY_UP output.
  - **macOS:** `resources/macos-globe-listener.swift` (Cocoa, tracks the **Fn / Globe** key and
    modifier flags) plus other Swift helpers; emits events on stdout.
- **Audio capture:** Web/renderer audio plus native mic helpers (`macos-mic-listener.swift`,
  `windows-mic-listener.c`) and a native **AEC (acoustic echo cancellation) helper**
  (`native/meeting-aec-helper/`, C++ using WebRTC-style AEC) for meeting capture. Uses
  `onnxruntime-node` and a downloaded Whisper VAD model.
- **Text insertion (per platform):** Dedicated **"fast-paste" native helpers**:
  `windows-fast-paste.c`, `resources/macos-fast-paste.swift`, `linux-fast-paste.c`
  (plus `nircmd` downloaded on Windows). I.e. clipboard+paste executed by a compiled helper per OS
  rather than a cross-platform Rust/JS lib. (Exact keystroke details per helper: unconfirmed beyond
  the clipboard-paste intent.)
- **Transcription engine:** Local **whisper.cpp** binaries (`download:whisper-cpp` script, and the
  package description literally says "using whisper.cpp"), plus **sherpa-onnx** (for NVIDIA
  **Parakeet**), a bundled **llama-server** (for the AI-agent / cleanup features), `onnxruntime-node`,
  Qdrant (vector store) and an embedding model for notes/RAG. Also offers cloud + BYOK paths.
  **GPU:** whisper.cpp/llama builds are downloaded per platform; GPU acceleration is build-dependent
  (unconfirmed which GPU backend ships on Windows).
- **Platform notes / known issues:** Heavy reliance on compiled native helpers means the build has
  many `compile:*` / `download:*` steps and a C compiler is needed (with prebuilt-binary download
  fallback so contributors without a compiler can still build). Large surface area (meetings, notes,
  agents, diarization) beyond pure dictation.
- **Lessons for Voxelle:**
  1. The **out-of-process native key-listener emitting KEY_DOWN/KEY_UP over stdout** is a clean,
     language-agnostic way to get reliable key-release across OSes; the Windows `WH_KEYBOARD_LL`
     and Linux evdev choices are exactly the low-level primitives for robust global push-to-talk.
     (We can get the same in-process in Rust via a low-level hook / evdev crate.)
  2. Note the explicit warning that `GetAsyncKeyState` is unreliable for the key currently in the
     hook callback — read key-up state from the hook event itself, not by polling.

---

## 3. Whispering (Epicenter)

- **Repo:** https://github.com/EpicenterHQ/epicenter (monorepo; app at `apps/whispering/`)
- **License:** **AGPL-3.0-or-later** (root `LICENSE` and `apps/whispering/package.json`
  `"license": "AGPL-3.0-or-later"`, with GNU AGPLv3 text in the app). **Not copyable** under our
  policy — learn only, reimplement originally.
- **Tech stack:** Tauri 2 (Rust shell in `apps/whispering/src-tauri/`) + SvelteKit/TypeScript
  frontend; data stored as plain Markdown + audio files in a Git-versioned folder (CRDT/local-first
  philosophy).
- **Important scoping caveat:** The **current in-repo milestone is a voice-note app**
  (record -> transcribe -> transform to files), **not** a system-wide dictation tool. Its README
  "What is not supported" explicitly lists: *"System-wide dictation: global shortcuts, push-to-talk,
  voice-activated..."* as **absent**. (Earlier, widely-known versions of Whispering did offer global
  shortcuts and local transcription; this survey reports what the current inspectable repo contains.)
- **Hotkey / global shortcut:** **Not present** in the current milestone (see caveat). The Tauri
  shell's Rust is focused on the Git-backed folder store and clipboard, not a global hotkey loop.
- **Audio capture:** Browser `MediaRecorder` in the webview (renderer-side audio), saved to the
  data folder. No native capture layer in the current shell (unconfirmed for historical versions).
- **Text insertion:** No active-window dictation/injection in the current milestone (it writes to
  files, not to other apps). Clipboard access exists in the Tauri shell for its own UI.
- **Transcription engine:** **OpenAI-compatible HTTP endpoint the user configures** (cloud or a
  local server), *not* a bundled whisper.cpp in this milestone. "Transformations" send transcripts
  to a prompt endpoint. GPU: N/A locally (depends on whatever endpoint the user points at).
- **Platform notes / known issues:** Explicitly a "development milestone, not a release"; the
  desktop window itself is noted as not yet hand-verified for some flows (tests only). No accounts,
  no sync, no system-wide dictation.
- **Lessons for Voxelle:**
  1. The **local-first, files-you-own data model** (each recording/transcript/transformation is a
     plain Markdown/audio file, versioned, no account/server) strongly aligns with Voxelle's
     privacy-first non-negotiables — a good model for how to persist history without a database lock-in.
  2. Separating **raw transcript** from **transformed/cleaned output** as distinct files mirrors our
     non-negotiable that AI output never overwrites the raw transcript.

---

## 4. WhisperWriter

- **Repo:** https://github.com/savbell/whisper-writer
- **License:** **GPL-3.0** (`LICENSE` is the GNU GPL v3). **Not copyable** under our policy — learn
  only, reimplement originally.
- **Tech stack:** **Python** app with a **PyQt5** UI (migrated from tkinter in 2024). Runs in the
  background as a script (`run.py`), small status window.
- **Hotkey / global shortcut (incl. key-release for push-to-talk):** Uses **`pynput`** as the
  keyboard/mouse backend. A `pynput.keyboard.Listener` is created with `on_press` / `on_release`
  callbacks (`src/key_listener.py`), translated into an internal `InputEvent.KEY_PRESS` /
  `KEY_RELEASE` + `KeyCode` enum. The explicit `KEY_RELEASE` event is what powers the
  **`hold_to_record`** mode (record while held, stop on release), alongside `continuous`,
  `voice_activity_detection`, and `press_to_toggle` modes. pynput gives one cross-platform
  key-up/down API (X11/Win32/macOS Quartz under the hood).
- **Audio capture:** **`sounddevice`** (PortAudio) for mic input; `webrtcvad-wheels` for VAD;
  `soundfile`/`av`/`ffmpeg-python` for audio handling.
- **Text insertion (per platform):** Configurable `input_method` (`src/input_simulation.py`):
  - **`pynput`** controller types the text character-by-character (press/release per char) — the
    cross-platform default.
  - **`ydotool`** and **`dotool`** are offered specifically for **Linux / Wayland** (where pynput
    key injection is unreliable). `dotool` is run as a long-lived subprocess fed via stdin.
  - Also depends on `pyperclip` (clipboard) and `PyGetWindow` (active-window targeting).
- **Transcription engine:** Local **`faster-whisper`** (CTranslate2 backend) by default, or
  **OpenAI API** (configurable, `src/transcription.py`). **GPU:** faster-whisper supports CUDA —
  README documents needing cuBLAS for CUDA 12 + cuDNN 8; `compute_type=int8` forces CPU, otherwise
  device is configurable (`cuda`/`cpu`) with a CPU fallback on init error. Good CUDA fit for a GTX 1080.
- **Platform notes / known issues:** Python packaging/dependency weight (large `requirements.txt`,
  CUDA libs for GPU). Wayland is the known problem area for key injection — hence the ydotool/dotool
  options. README candidly notes much of the early code/README was AI-generated.
- **Lessons for Voxelle:**
  1. A clean **recording-mode taxonomy** to copy conceptually: `hold_to_record` (release-to-stop),
     `press_to_toggle`, `continuous`, and `voice_activity_detection`. Voxelle should expose the same
     set; they all fall out naturally once you have reliable key-press/key-release events.
  2. **faster-whisper (CTranslate2)** is a strong CPU path and a straightforward CUDA path; worth
     benchmarking against whisper.cpp+Vulkan on the GTX 1080 before committing to one ASR backend.

---

## Cross-project lessons for Voxelle

**Hotkey / key-release strategy (push-to-talk):**
- All four expose an explicit **key-up (release) event**, which is the core requirement for
  push-to-talk. Press-oriented "global shortcut" plugins alone are not enough.
- Two proven patterns:
  - *In-process low-level hook* (Handy: `handy-keys`/`rdev` on a dedicated owner thread) — best fit
    for a Tauri+Rust app like Voxelle.
  - *Out-of-process native helper emitting KEY_DOWN/KEY_UP on stdout* (OpenWhispr: Win32
    `WH_KEYBOARD_LL`, Linux evdev, macOS Globe/Quartz) — more moving parts but very robust and
    language-agnostic.
- Concrete gotcha (from OpenWhispr source): on Windows, read key-up from the hook event itself;
  `GetAsyncKeyState` is unreliable for the key that triggered the current callback.
- Recommendation: In-process Rust low-level hook on a dedicated thread with an explicit
  press/release state machine; offer hold-to-record, toggle, continuous, and VAD modes.

**Text insertion strategy (per platform):**
- **Windows / macOS:** clipboard + synthetic paste (Ctrl+V / Cmd+V) is the reliable default
  (Handy, OpenWhispr). Use **platform virtual key codes** for the paste, and on macOS resolve the
  physical "V" via TIS/Carbon so non-US layouts work. Save and restore the user's clipboard.
- **Linux is the hard case** and needs runtime branching:
  - Wayland: `wtype` (not on KDE/GNOME due to missing `zwp_virtual_keyboard_manager_v1`), else
    `ydotool` / `dotool`; `wl-copy` for clipboard.
  - X11: `xdotool`, else `ydotool`.
  - Detect session type + compositor, pick the tool, and give the user an override. (Handy and
    WhisperWriter both ship ydotool/dotool/wtype options precisely because no single method works.)
- Keep clipboard-paste as default, with an optional direct-typing path for apps that block paste.

**ASR engine choice:**
- Dominant local engines: **whisper.cpp** (Handy via `transcribe-cpp`; OpenWhispr), **faster-whisper /
  CTranslate2** (WhisperWriter), and **sherpa-onnx / ONNX** for **NVIDIA Parakeet** (Handy, OpenWhispr).
- For Voxelle (Tauri + Rust): whisper.cpp via a Rust binding (as Handy does) keeps everything in-process
  and avoids a Python runtime. Parakeet via ONNX is a strong CPU-optimized alternative with auto
  language detection. VAD (Silero / webrtcvad / earshot) in front of ASR is universal — add it.

**GPU / CPU (dev machine: Windows + GTX 1080, 8 GB):**
- Best match on our hardware is **whisper.cpp with the Vulkan backend on Windows** — exactly what
  Handy ships for Windows x86_64 (Metal on macOS, CPU fallback elsewhere). Vulkan avoids a hard CUDA
  toolkit dependency.
- **faster-whisper + CUDA** (WhisperWriter) is a solid alternative but requires cuBLAS/cuDNN install.
- Always ship a **CPU fallback** (Parakeet/CPU whisper) for machines without a usable GPU, and pick
  the backend at runtime.

**Architecture fit:** Handy is the closest reference (Tauri 2 + Rust + React/TS + whisper, MIT) and
is the most directly instructive — and, being MIT, the only one of the four we could copy from
(with attribution). OpenWhispr (MIT) is instructive for native key/insertion helpers. Whispering
(AGPL) and WhisperWriter (GPL) are **read-only / learn-only** under our policy.
