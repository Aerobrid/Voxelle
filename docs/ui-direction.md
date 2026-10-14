# UI direction — Voxelle

Original, sleek, dark-first design. Derived from the Voxelle logo
(`assets/branding/voxelle-logo-master.svg`): a rounded-square mark holding a white audio
**waveform** (vertical rounded bars of varying height) on a deep navy field. We reuse that
language — rounded geometry, a calm near-black navy canvas, and the waveform as a recurring motif
(recording indicator, level meter, section accents). We do **not** copy the reference app's layout,
icons, palette, copy, sounds, or onboarding.

## Palette (derived from the logo)
Logo source colors: fill `#101427`, stroke `#111729`, mark `#FFFFFF`. The navy sits around hue
~225° (blue). The system is built outward from it.

### Core neutrals (dark theme — default)
| Token | Hex | Use |
|---|---|---|
| `--bg-0` (brand ink) | `#101427` | App background base (matches logo field) |
| `--bg-1` | `#141A30` | App canvas |
| `--surface-1` | `#1A2140` | Cards, panels |
| `--surface-2` | `#222A4D` | Raised / hover surfaces |
| `--border` | `#2C3560` | Hairlines, dividers |
| `--text-1` | `#F4F6FF` | Primary text (the logo white, softened) |
| `--text-2` | `#AEB6D6` | Secondary text |
| `--text-3` | `#6E769B` | Muted / disabled |

### Accents
| Token | Hex | Meaning |
|---|---|---|
| `--accent` | `#5B8CFF` | Primary interactive (azure-indigo from brand hue) |
| `--accent-2` | `#8A7CFF` | Waveform gradient end (violet), secondary highlight |
| `--recording` | `#FF4D5E` | Live microphone capture active (never ambiguous) |
| `--success` | `#3DD68C` | Inserted / verified / local-only |
| `--warning` | `#FFB454` | Copied-only, or a network-capable action |
| `--danger` | `#FF5466` | Failed / error state |

### Semantic privacy chips (always visible on relevant actions)
- **Local** — `--success` dot + "Local". Default for all P0 work.
- **Network** — `--warning` dot + "Can use network". Only shown when a feature may leave the device.
- **Recording** — `--recording` pulsing dot while the mic is open.
These map to principles.md #4 (local/remote indicator on every operation that could use the network)
and the privacy dashboard.

A light theme is **planned** (invert neutrals, keep accents); not built in P0.

## Typography
- **UI / text:** Inter (SIL OFL 1.1 — permissive; recorded in dependencies.md). System fallback:
  `-apple-system, Segoe UI, Roboto, sans-serif`.
- **Transcripts / numeric / technical:** JetBrains Mono (SIL OFL 1.1). Monospace makes edit points and
  timestamps legible and visually separates the raw transcript from AI-edited text.
- Weights: 400 body, 500 UI labels, 600 headings. Generous line-height (1.5) for transcript reading.

## Shape, spacing, motion
- Radius scale: 8 / 12 / 16 px; the app icon and primary buttons echo the logo's rounded square.
- Spacing: 4px base grid (4/8/12/16/24/32).
- Motion: subtle, purposeful. The recording indicator animates the waveform bars from real input
  level (ties to the level meter). Respect "reduce motion" accessibility setting.
- Elevation: flat with hairline borders + soft shadows on raised surfaces; no skeuomorphism.

## Layout shell
Left nav rail (icon + label) → sections from principles.md: Dashboard, History, Vocabulary Coach,
AI Assistant, Notes, Audio Import, Model Manager, Settings, Privacy. A persistent status strip shows
current state (idle/recording/processing), the active hotkey, selected mic, and the Local/Network
chip. P0 ships Dashboard + History + Model Manager + Settings + Privacy as the minimum; the rest are
visible but labeled **planned** until their priority lands (never faked — principles.md #8).

## Icon derivation (from the master SVG)
App icon, tray icon, installer, favicon all derive from `voxelle-logo-master.svg`. The color version
is used at ≥32px. A **small monochrome tray icon** likely cannot be derived cleanly from the detailed
5-bar mark at 16px — flagged for a simplified 2–3 bar monochrome glyph; to be confirmed when icons are
generated (see implementation-roadmap). Derivation (SVG → PNG/ICO) needs a rasterizer dependency,
recorded in dependencies.md when added.
