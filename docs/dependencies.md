# Dependencies & licenses

Every dependency needs a one-line justification + license check before use (CLAUDE.md). Permissive
or compatible licenses only (MIT / Apache-2.0 / BSD / ISC / MPL-2.0 / SIL-OFL for fonts). No GPL/AGPL
in shipped code. Models get their own license check before bundling or download.

Status legend: **planned** = intended, not yet added to a manifest · **added** = in a lockfile ·
**verified** = license confirmed from the package's own LICENSE.

## Runtime toolchain (host machine, not shipped)
| Tool | Version | License | Notes |
|---|---|---|---|
| Node | 24.11.1 | — | Frontend build/test (Vite). verified present |
| Rust/cargo | 1.96.0 (msvc) | — | Core build. verified present |
| CMake | 4.2.1 | — | Builds whisper.cpp. verified present |
| MSVC C++ Build Tools | — | — | **MISSING — install in progress.** Required to link Rust + build whisper.cpp |
| WebView2 Runtime | 154.x | — | Tauri renderer on Windows. verified present |

## Rust crates (planned for P0 — added to Cargo.toml as each slice lands)
| Crate | Purpose | License | Status |
|---|---|---|---|
| tauri (2.x) | Desktop shell, IPC, windowing, tray | MIT/Apache-2.0 | planned |
| tauri-plugin-global-shortcut | Toggle/cancel global hotkeys | MIT/Apache-2.0 | planned |
| cpal | Cross-platform audio capture | Apache-2.0 | planned |
| whisper-rs | Rust bindings to whisper.cpp (local ASR) | Unlicense/MIT | planned — confirm bundled whisper.cpp license (MIT) |
| rusqlite (bundled) | SQLite storage, migrations, params | MIT | planned |
| serde / serde_json | IPC + settings serialization | MIT/Apache-2.0 | planned |
| thiserror | Error types across modules | MIT/Apache-2.0 | planned |
| rdev *(candidate)* | Low-level key state for push-to-talk key-release | MIT | planned — evaluate vs platform hooks in 0.1 |
| arboard *(candidate)* | Clipboard get/set for paste-insertion + restore | MIT/Apache-2.0 | planned |
| enigo *(candidate)* | Synthesize paste keystroke / fallback input | MIT | planned — evaluate vs windows-rs SendInput |
| windows *(Windows only)* | UIA/SendInput/DPAPI via Win32 | MIT/Apache-2.0 | planned |
| sha2 | SHA-256 model integrity verification | MIT/Apache-2.0 | planned |
| chrono | Timestamps for history entries (RFC3339) | MIT/Apache-2.0 | added (default-features off; `clock`+`std`) |

Candidates marked *(candidate)* are chosen at implementation time; the pick + rejected options are
logged in decisions.md. GPL-licensed input libs (if any) are excluded.

## Frontend packages (planned for P0)
| Package | Purpose | License | Status |
|---|---|---|---|
| react / react-dom | UI | MIT | planned |
| typescript | Types | Apache-2.0 | planned |
| vite + @vitejs/plugin-react | Build/dev server | MIT | planned |
| @tauri-apps/api | Renderer↔core IPC (no direct OS access) | MIT/Apache-2.0 | planned |
| vitest | Frontend unit tests | MIT | planned |
| zustand *(candidate)* | Small state store for session/state-machine mirror | MIT | planned |

## Fonts
| Font | Use | License | Status |
|---|---|---|---|
| Inter | UI text | SIL OFL 1.1 | planned — bundle locally (offline; no CDN) |
| JetBrains Mono | Transcripts/technical | SIL OFL 1.1 | planned — bundle locally |

Fonts are bundled locally, never fetched from a CDN at runtime (offline-by-default; no network in the
core loop).

## Models (never committed; downloaded only with consent + checksum)
| Model | Purpose | License | Status |
|---|---|---|---|
| whisper.cpp GGUF (base/small/medium, quantized) | Local ASR | MIT (model weights by OpenAI under MIT) | planned — verify SHA-256 against pinned values before use |

## Known advisories (tracked)
- `npm audit` (2026-10-13) reports 6 advisories (2 critical, 1 high, 3 moderate) — **all in the
  dev/test toolchain** (`vitest`, `@vitest/mocker`, `vite`, `vite-node`, `esbuild`, `tinypool`).
  They affect the local dev server and test runner, not the shipped app bundle. Fixing requires
  major bumps (vite 7 / vitest 3). Deferred to the CI/audit slice; not shipped, so not blocking.
  A production `npm audit --omit=dev` is the gate before release.

## Tools we deliberately avoid shipping
- No pickle / remote-code model formats (security.md §6) — GGUF/safetensors/ONNX only.
- No telemetry/analytics/crash SDKs (PostHog, Sentry, etc.) — forbidden by non-negotiables.
- No cloud ASR/LLM SDKs in the core loop.
