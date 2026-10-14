# Dependencies & licenses

Every dependency needs a one-line justification + license check before use (CLAUDE.md). Permissive
or compatible licenses only (MIT / Apache-2.0 / BSD / ISC / MPL-2.0 / Unlicense / SIL-OFL for fonts).
No GPL/AGPL in shipped code. Models get their own license check before bundling or download.

Status legend: **planned** = intended, not yet added to a manifest · **added** = in a lockfile ·
**verified** = license confirmed from the package's own manifest/LICENSE file.

## Build toolchain (developer machine, not shipped)
| Tool | Version | License | Why |
|---|---|---|---|
| Node / npm | 24.11.1 / 11.6.2 | — | Frontend build/test (Vite). verified present |
| Rust/cargo | 1.96.0 (msvc) | — | Core build. verified present |
| Visual Studio C++ tools (VS 18) | MSVC 14.51 | proprietary (build tool only) | Rust MSVC linker + whisper.cpp C/C++ compile. verified present 2026-10-14 |
| CMake | 4.2.1 | BSD-3 | Builds whisper.cpp. verified present |
| Ninja (bundled with VS) | — | Apache-2.0 | CMake generator; avoids the cmake-crate /O2 bug and MAX_PATH (see decisions 2026-10-14) |
| LLVM / libclang | 23.1.3 | Apache-2.0 WITH LLVM-exception | bindgen for whisper-rs-sys (bundled bindings are Linux-only). Installed 2026-10-14 via winget |
| LunarG Vulkan SDK | 1.4.363.0 | Apache-2.0 (components per SDK `Licenses/`) | Headers + `glslc` to build whisper.cpp's Vulkan backend. Installed 2026-10-14 via winget. Not shipped |
| WebView2 Runtime | 154.x | Microsoft (system component) | Tauri renderer on Windows. verified present |

## Rust crates
| Crate | Version | Purpose (one line) | License | Status |
|---|---|---|---|---|
| tauri | 2.12 | Desktop shell, IPC, windows (main + overlay) | MIT OR Apache-2.0 | added |
| serde / serde_json | 1 | IPC + settings serialization | MIT OR Apache-2.0 | added |
| thiserror | 1 | Error types across modules | MIT OR Apache-2.0 | added |
| chrono | 0.4 | History timestamps (RFC 3339); default features off | MIT OR Apache-2.0 | added |
| cpal | 0.18.2 | Cross-platform microphone capture | Apache-2.0 | added, verified 2026-10-14 |
| whisper-rs | 0.16.0 | Rust bindings to whisper.cpp (local ASR) | Unlicense | added, verified 2026-10-14 |
| whisper-rs-sys (bundles whisper.cpp + ggml) | 0.15.0 | Builds/links whisper.cpp | Unlicense; whisper.cpp/ggml MIT | added, verified (whisper.cpp `LICENSE` = MIT) |
| sha2 | 0.11.1 | SHA-256 model integrity verification | MIT OR Apache-2.0 | added, verified |
| windows-sys (Windows only) | 0.61.2 | SendInput, clipboard, LL keyboard hook, token elevation | MIT OR Apache-2.0 | added, verified; already in tree via Tauri (no new code compiled) |
| hound (dev-dependency) | 3.5.1 | Read WAV corpus in the ASR benchmark tool only | Apache-2.0 | added (dev only, not shipped) |
| rusqlite (bundled) | — | SQLite storage, migrations, params | MIT | planned (storage slice) |

Rejected/not used: `rdev`, `enigo`, `arboard`, `tauri-plugin-global-shortcut` — the Windows
adapters are ~400 lines over `windows-sys`, which was already a transitive dependency; this gives
key-up detection, injected-event filtering, and full clipboard save/restore that the candidates do
not all provide. Revisit for macOS/Linux adapters.

**Vulkan at runtime:** the `gpu-vulkan` build links `vulkan-1.dll` (the Khronos loader, Apache-2.0)
which ships with GPU drivers; we do not redistribute it. Shaders are compiled at build time by
`glslc`; no SDK component is shipped.

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
| whisper.cpp GGML `.bin` (base, base.en, small, small.en, medium, large-v3-turbo; Q5 quantized) | Local ASR | MIT (OpenAI Whisper weights, MIT; GGML conversions in the whisper.cpp model repo, MIT) | catalog with pinned SHA-256 in `src-tauri/src/models/mod.rs`; base/small/medium/large-v3-turbo downloaded for benchmarking 2026-10-14, all four hashes matched |

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
