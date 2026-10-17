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

---

## v2 direction (2026-10-15) — "quiet instrument"
User feedback on v1: functional but looked generic/"vibe-coded" (boxed cards everywhere, debug-ish
text, a heavy native title bar). v2 keeps the logo-derived palette and waveform motif and changes
the *structure and finish*. It is designed from our own brand, not from any other app; no
screenshots of the reference app are used as a spec.

**Principles**
1. **Frameless, calm chrome.** Native decorations off; our own 44 px title bar (drag region, brand
   mark, live state, window buttons drawn as thin line glyphs). Windows 11 still supplies the
   rounded corners and shadow. Close = hide to tray (setting).
2. **One focal object per screen.** Home is built around the *voice orb*: a round control whose
   inner waveform bars are driven by the real mic level, ringed by the brand gradient. Everything
   else on the page is secondary.
3. **Fewer boxes.** Hierarchy from type, spacing and hairline dividers; cards only where content
   is a distinct object (a transcript, a model). Settings are grouped rows: label + one-line
   explanation on the left, control on the right; toggles are switches.
4. **Truthful, human copy.** No raw state names ("idle") or developer metrics up front; timings
   live in a "Details" disclosure. Planned features are grouped under "Coming soon" in the
   sidebar, still clearly labeled.
5. **Real numbers only.** Home stats (words / dictations today) are computed from local history;
   "—" when there is nothing yet.

**Refined tokens** (supersede v1 where they differ)
| Token | Value | Use |
|---|---|---|
| `--bg` | `#0E1224` | Window canvas (a step below the logo ink for depth) |
| `--bg-side` | `#0B0F1E` | Sidebar + title bar |
| `--ink` | `#101427` | Logo field color; orb core |
| `--surface` | `rgba(255,255,255,0.035)` | Object cards |
| `--surface-hover` | `rgba(255,255,255,0.06)` | Hover rows |
| `--line` | `rgba(255,255,255,0.07)` | Hairlines |
| `--grad` | `linear-gradient(135deg, #5B8CFF, #8A7CFF)` | Waveform gradient: orb ring, primary button, active nav marker |
| radii | 8 / 12 / 18 / 999 | controls / rows / cards / pills + orb |

**Typography (changed):** the platform UI font (Segoe UI Variable on Windows 11, SF on macOS,
system sans on Linux) instead of bundling Inter: native rendering, zero bytes, nothing to fetch.
Mono: Cascadia Mono / JetBrains Mono if installed, else the platform monospace. Scale: 26/600 page
title, 15/600 section, 14/400 body, 12.5 meta.

**Motion:** 120–180 ms ease-out for hovers and state changes; the orb ring breathes slowly only
while recording; everything static under `prefers-reduced-motion`.

**Tray glyph:** still the full-colour app icon. A simplified monochrome 3-bar glyph for 16 px is
proposed (derived, originals untouched) and pending user approval.

## v2.1 (2026-10-17) — feedback pass
User feedback: the key chips looked like buttons next to a separate "Change" button (confusing,
jarring); dropdowns opened as square OS lists; errors were small red text; wanted a nicer font.
- **Typography (changed again):** bundle **Inter** (UI) and **JetBrains Mono** (keys, numbers,
  paths), both OFL 1.1, served from the app (no network). Same look on every OS; replaces the
  platform-font choice of v2.
- **Shortcut field:** one control — a quiet field holding flat mono keycaps plus a small pencil;
  clicking it opens a dialog that listens. Turning a shortcut off is a small × beside it (a spacer
  keeps fields aligned when there is none).
- **Dialogs for decisions, toasts for confirmations:** a refused shortcut explains why in the
  dialog (red outline + alert glyph, "Try again"); successful saves, copies and model switches are a
  short bottom-center pill toast with a check glyph. Error toasts stay twice as long.
- **Dropdown:** our own listbox with the card radius (12 px), raised surface, check on the chosen
  option, flips upward near the window edge.
- **Developer numbers off Home:** timings and the slow-mic note moved to a Stats page; Home keeps
  words, dictations and words-per-minute today.
