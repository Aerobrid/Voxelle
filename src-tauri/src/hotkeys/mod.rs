//! Global hotkeys (P0 §0.1): toggle, push-to-talk (needs key-release, so a low-level key hook
//! rather than a press-only shortcut API), and cancel. Parsing and matching are pure and unit
//! tested; `windows.rs` feeds real key events in. Key codes are Windows virtual-key codes, which
//! other platform adapters translate into.

pub mod capture;
#[cfg(windows)]
mod windows;

use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::dictation;
use crate::session::SessionState;

// ---------- bindings ----------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
}

impl Mods {
    fn count(&self) -> usize {
        [self.ctrl, self.shift, self.alt, self.win].iter().filter(|b| **b).count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// Modifiers + one non-modifier key, e.g. Ctrl+Shift+Space. Matches exact modifiers.
    Combo { mods: Mods, vk: u16 },
    /// A single side-specific modifier held on its own, e.g. RightCtrl (push-to-talk friendly).
    ModifierOnly { vk: u16 },
    /// Two or more modifiers held together with no other key, e.g. Ctrl+Win (either side).
    ModifierChord { mods: Mods },
}

impl Binding {
    /// Holding the Windows key and releasing it without another key opens the Start menu, so
    /// bindings that leave Win held need a masking key event when they fire.
    fn holds_win(&self) -> bool {
        match *self {
            Binding::ModifierOnly { vk } => matches!(vk, vk::LWIN | vk::RWIN),
            Binding::ModifierChord { mods } | Binding::Combo { mods, .. } => mods.win,
        }
    }
}

pub mod vk {
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;
    pub const PAUSE: u16 = 0x13;
    pub const CAPITAL: u16 = 0x14;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21;
    pub const NEXT: u16 = 0x22;
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const SNAPSHOT: u16 = 0x2C;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const LWIN: u16 = 0x5B;
    pub const RWIN: u16 = 0x5C;
    pub const APPS: u16 = 0x5D;
    pub const NUMPAD0: u16 = 0x60;
    pub const MULTIPLY: u16 = 0x6A;
    pub const ADD: u16 = 0x6B;
    pub const SUBTRACT: u16 = 0x6D;
    pub const DECIMAL: u16 = 0x6E;
    pub const DIVIDE: u16 = 0x6F;
    pub const F1: u16 = 0x70;
    pub const SCROLL: u16 = 0x91;
    pub const LSHIFT: u16 = 0xA0;
    pub const RSHIFT: u16 = 0xA1;
    pub const LCONTROL: u16 = 0xA2;
    pub const RCONTROL: u16 = 0xA3;
    pub const LMENU: u16 = 0xA4;
    pub const RMENU: u16 = 0xA5;
    pub const OEM_1: u16 = 0xBA; // ; :
    pub const OEM_PLUS: u16 = 0xBB; // = +
    pub const OEM_COMMA: u16 = 0xBC;
    pub const OEM_MINUS: u16 = 0xBD;
    pub const OEM_PERIOD: u16 = 0xBE;
    pub const OEM_2: u16 = 0xBF; // / ?
    pub const OEM_3: u16 = 0xC0; // ` ~
    pub const OEM_4: u16 = 0xDB; // [ {
    pub const OEM_5: u16 = 0xDC; // \ |
    pub const OEM_6: u16 = 0xDD; // ] }
    pub const OEM_7: u16 = 0xDE; // ' "
    /// Unassigned virtual key, sent so that releasing Win after a shortcut does not open Start.
    pub const MASK: u16 = 0xE8;
}

fn is_modifier(code: u16) -> bool {
    matches!(
        code,
        vk::SHIFT | vk::CONTROL | vk::MENU | vk::LWIN | vk::RWIN | vk::LSHIFT | vk::RSHIFT
            | vk::LCONTROL | vk::RCONTROL | vk::LMENU | vk::RMENU
    )
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("empty shortcut")]
    Empty,
    #[error("unknown key '{0}'")]
    UnknownKey(String),
    #[error("use one key plus optional modifiers (Ctrl+Shift+Space), two or more modifiers together (Ctrl+Win), or one side-specific modifier (RightCtrl)")]
    NeedsOneKey,
    #[error("'{0}' cannot be used as a shortcut key")]
    Reserved(String),
}

/// Canonical display name first, then accepted aliases. Used for both parsing and formatting.
const KEY_NAMES: &[(u16, &[&str])] = &[
    (vk::SPACE, &["Space"]),
    (vk::ESCAPE, &["Escape", "Esc"]),
    (vk::INSERT, &["Insert"]),
    (vk::DELETE, &["Delete", "Del"]),
    (vk::HOME, &["Home"]),
    (vk::END, &["End"]),
    (vk::PRIOR, &["PageUp"]),
    (vk::NEXT, &["PageDown"]),
    (vk::LEFT, &["Left"]),
    (vk::RIGHT, &["Right"]),
    (vk::UP, &["Up"]),
    (vk::DOWN, &["Down"]),
    (vk::PAUSE, &["Pause"]),
    (vk::SCROLL, &["ScrollLock"]),
    (vk::CAPITAL, &["CapsLock"]),
    (vk::SNAPSHOT, &["PrintScreen"]),
    (vk::APPS, &["Menu", "Apps"]),
    (vk::OEM_3, &["Backquote", "`"]),
    (vk::OEM_MINUS, &["Minus", "-"]),
    (vk::OEM_PLUS, &["Equal", "="]),
    (vk::OEM_4, &["BracketLeft", "["]),
    (vk::OEM_6, &["BracketRight", "]"]),
    (vk::OEM_5, &["Backslash", "\\"]),
    (vk::OEM_1, &["Semicolon", ";"]),
    (vk::OEM_7, &["Quote", "'"]),
    (vk::OEM_COMMA, &["Comma", ","]),
    (vk::OEM_PERIOD, &["Period", "."]),
    (vk::OEM_2, &["Slash", "/"]),
    (vk::MULTIPLY, &["NumpadMultiply"]),
    (vk::ADD, &["NumpadAdd"]),
    (vk::SUBTRACT, &["NumpadSubtract"]),
    (vk::DECIMAL, &["NumpadDecimal"]),
    (vk::DIVIDE, &["NumpadDivide"]),
    (vk::LCONTROL, &["LeftCtrl", "LCtrl"]),
    (vk::RCONTROL, &["RightCtrl", "RCtrl"]),
    (vk::LSHIFT, &["LeftShift", "LShift"]),
    (vk::RSHIFT, &["RightShift", "RShift"]),
    (vk::LMENU, &["LeftAlt", "LAlt"]),
    (vk::RMENU, &["RightAlt", "RAlt", "AltGr"]),
    (vk::LWIN, &["LeftWin", "LWin"]),
    (vk::RWIN, &["RightWin", "RWin"]),
];

/// Keys that would submit forms / move focus / delete text if a shortcut swallowed them.
const RESERVED: &[(u16, &[&str])] = &[
    (vk::RETURN, &["Enter", "Return"]),
    (vk::TAB, &["Tab"]),
    (vk::BACK, &["Backspace"]),
];

fn named_key(name: &str) -> Option<u16> {
    if let Some((code, _)) = KEY_NAMES
        .iter()
        .find(|(_, names)| names.iter().any(|n| n.eq_ignore_ascii_case(name)))
    {
        return Some(*code);
    }
    let n = name.to_ascii_lowercase();
    let b = n.as_bytes();
    if b.len() == 1 && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit()) {
        return Some(b[0].to_ascii_uppercase() as u16);
    }
    if let Some(num) = n.strip_prefix("numpad").and_then(|d| d.parse::<u16>().ok()) {
        if num <= 9 {
            return Some(vk::NUMPAD0 + num);
        }
    }
    if let Some(num) = n.strip_prefix('f').and_then(|d| d.parse::<u16>().ok()) {
        if (1..=24).contains(&num) {
            return Some(vk::F1 + num - 1);
        }
    }
    None
}

/// Display name for a virtual key, or None if shortcuts cannot use it.
pub fn key_name(code: u16) -> Option<String> {
    if let Some((_, names)) = KEY_NAMES.iter().find(|(c, _)| *c == code) {
        return Some(names[0].to_string());
    }
    match code {
        0x30..=0x39 | 0x41..=0x5A => Some((code as u8 as char).to_string()),
        c if (vk::NUMPAD0..=vk::NUMPAD0 + 9).contains(&c) => Some(format!("Numpad{}", c - vk::NUMPAD0)),
        c if (vk::F1..vk::F1 + 24).contains(&c) => Some(format!("F{}", c - vk::F1 + 1)),
        _ => None,
    }
}

/// Name of a reserved key (Enter/Tab/Backspace), if `code` is one.
pub fn reserved_name(code: u16) -> Option<&'static str> {
    RESERVED.iter().find(|(c, _)| *c == code).map(|(_, names)| names[0])
}

/// Parse "Ctrl+Shift+Space", "CommandOrControl+Shift+Space", "RightCtrl", "Ctrl+Win", "F9"
/// (case-insensitive).
pub fn parse_binding(s: &str) -> Result<Binding, ParseError> {
    let parts: Vec<&str> = s.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut mods = Mods::default();
    let mut keys = Vec::new();
    for p in &parts {
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" | "cmdorctrl" => mods.ctrl = true,
            "shift" => mods.shift = true,
            "alt" | "option" => mods.alt = true,
            "win" | "super" | "meta" | "cmd" | "command" => mods.win = true,
            _ => {
                let reserved = RESERVED
                    .iter()
                    .find(|(_, names)| names.iter().any(|n| n.eq_ignore_ascii_case(p)));
                if let Some((_, names)) = reserved {
                    return Err(ParseError::Reserved(names[0].into()));
                }
                keys.push(named_key(p).ok_or_else(|| ParseError::UnknownKey((*p).into()))?)
            }
        }
    }
    match keys.as_slice() {
        [] if mods.count() >= 2 => Ok(Binding::ModifierChord { mods }),
        [k] if is_modifier(*k) && mods == Mods::default() => Ok(Binding::ModifierOnly { vk: *k }),
        [k] if !is_modifier(*k) => Ok(Binding::Combo { mods, vk: *k }),
        _ => Err(ParseError::NeedsOneKey),
    }
}

fn mods_names(m: Mods) -> Vec<String> {
    [(m.ctrl, "Ctrl"), (m.win, "Win"), (m.alt, "Alt"), (m.shift, "Shift")]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, n)| n.to_string())
        .collect()
}

/// Canonical text for a binding; `parse_binding(&format_binding(b)) == Ok(b)`.
pub fn format_binding(b: &Binding) -> String {
    match *b {
        Binding::Combo { mods, vk } => {
            let mut parts = mods_names(mods);
            parts.push(key_name(vk).unwrap_or_else(|| format!("0x{vk:02X}")));
            parts.join("+")
        }
        Binding::ModifierOnly { vk } => key_name(vk).unwrap_or_default(),
        Binding::ModifierChord { mods } => mods_names(mods).join("+"),
    }
}

#[derive(Debug, Clone, Default)]
pub struct Bindings {
    pub toggle: Option<Binding>,
    pub push_to_talk: Option<Binding>,
    pub cancel: Option<Binding>,
}

impl Bindings {
    pub fn from_settings(s: &crate::storage::Settings) -> Result<Self, String> {
        let parse = |label: &str, v: &Option<String>| -> Result<Option<Binding>, String> {
            match v.as_deref().map(str::trim) {
                None | Some("") => Ok(None),
                Some(x) => parse_binding(x).map(Some).map_err(|e| format!("{label} shortcut: {e}")),
            }
        };
        let b = Self {
            toggle: parse("Toggle", &Some(s.toggle_hotkey.clone()))?,
            push_to_talk: parse("Push-to-talk", &s.push_to_talk_hotkey)?,
            cancel: parse("Cancel", &s.cancel_hotkey)?,
        };
        if b.toggle.is_some() && b.toggle == b.push_to_talk {
            return Err("Toggle and push-to-talk cannot use the same shortcut".into());
        }
        Ok(b)
    }
}

// ---------- matcher ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Toggle,
    PttPressed,
    PttReleased,
    /// Another key was pressed while a modifier-only push-to-talk key was held: the user was
    /// typing a shortcut (e.g. RightCtrl+C), not dictating.
    PttChord,
    Cancel,
}

#[derive(Debug, Default)]
pub struct Matcher {
    held: Mods,
    ptt_down: bool,
    /// Modifier-only / chord toggle is held and no other key was pressed since (fires on release).
    toggle_mod_clean: Option<bool>,
    /// Non-modifier keys whose down event we swallowed; their up event must be swallowed too.
    swallowed: Vec<u16>,
}

fn mod_slot(code: u16) -> Option<fn(&mut Mods) -> &mut bool> {
    match code {
        vk::CONTROL | vk::LCONTROL | vk::RCONTROL => Some(|m| &mut m.ctrl),
        vk::SHIFT | vk::LSHIFT | vk::RSHIFT => Some(|m| &mut m.shift),
        vk::MENU | vk::LMENU | vk::RMENU => Some(|m| &mut m.alt),
        vk::LWIN | vk::RWIN => Some(|m| &mut m.win),
        _ => None,
    }
}

pub struct Outcome {
    pub actions: Vec<Action>,
    /// Hide this key event from the focused app.
    pub swallow: bool,
    /// A binding that holds Win just fired: send a masking key so releasing Win does not open Start.
    pub mask_win: bool,
}

impl Matcher {
    /// Feed one physical key event. `session_active` = recording or processing (cancel applies).
    pub fn on_key(&mut self, b: &Bindings, code: u16, down: bool, session_active: bool) -> Outcome {
        let mut actions = Vec::new();
        let mut swallow = false;
        let mut mask_win = false;
        let was_held = self.held;
        if let Some(slot) = mod_slot(code) {
            *slot(&mut self.held) = down;
        }
        // A modifier chord (e.g. Ctrl+Win) just became exactly what is held.
        let chord_completed = |mods: Mods| down && is_modifier(code) && self.held == mods && was_held != mods;

        match b.push_to_talk {
            Some(Binding::ModifierOnly { vk }) => {
                if code == vk {
                    if down && !self.ptt_down {
                        self.ptt_down = true;
                        mask_win |= b.push_to_talk.is_some_and(|p| p.holds_win());
                        actions.push(Action::PttPressed);
                    } else if !down && self.ptt_down {
                        self.ptt_down = false;
                        actions.push(Action::PttReleased);
                    }
                } else if down && self.ptt_down && !is_modifier(code) {
                    self.ptt_down = false;
                    actions.push(Action::PttChord);
                }
            }
            Some(Binding::ModifierChord { mods }) => {
                if !self.ptt_down && chord_completed(mods) {
                    self.ptt_down = true;
                    mask_win |= mods.win;
                    actions.push(Action::PttPressed);
                } else if self.ptt_down && !down && mod_slot(code).is_some() && self.held != mods {
                    self.ptt_down = false;
                    actions.push(Action::PttReleased);
                } else if self.ptt_down && down && code != vk::MASK && (!is_modifier(code) || self.held != mods) {
                    // Ctrl+Win+D, or an extra modifier: the user is using a system shortcut.
                    self.ptt_down = false;
                    actions.push(Action::PttChord);
                }
            }
            Some(Binding::Combo { mods, vk }) => {
                if down && code == vk && was_held == mods && !self.ptt_down {
                    self.ptt_down = true;
                    swallow = true;
                    mask_win |= mods.win;
                    actions.push(Action::PttPressed);
                } else if down && code == vk && self.ptt_down {
                    swallow = true; // key repeat while held
                } else if !down && self.ptt_down && (code == vk || mod_slot(code).is_some_and(|_| self.held != mods)) {
                    self.ptt_down = false;
                    actions.push(Action::PttReleased);
                }
            }
            None => {}
        }

        match b.toggle {
            Some(Binding::Combo { mods, vk }) => {
                if down && code == vk && was_held == mods {
                    swallow = true;
                    if !self.swallowed.contains(&code) {
                        mask_win |= mods.win;
                        actions.push(Action::Toggle); // ignore auto-repeat
                    }
                }
            }
            Some(Binding::ModifierOnly { vk }) => {
                // Fire on release, and only if no other key was pressed meanwhile (not part of a chord).
                if code == vk {
                    if down && self.toggle_mod_clean.is_none() {
                        self.toggle_mod_clean = Some(true);
                        mask_win |= b.toggle.is_some_and(|t| t.holds_win());
                    } else if !down && self.toggle_mod_clean.take() == Some(true) {
                        actions.push(Action::Toggle);
                    }
                } else if down && self.toggle_mod_clean.is_some() {
                    self.toggle_mod_clean = Some(false);
                }
            }
            Some(Binding::ModifierChord { mods }) => {
                // Fire when the chord is released, unless another key joined it in between.
                if chord_completed(mods) && self.toggle_mod_clean.is_none() {
                    self.toggle_mod_clean = Some(true);
                    mask_win |= mods.win;
                } else if self.toggle_mod_clean.is_some() {
                    if down && code != vk::MASK && (!is_modifier(code) || self.held != mods) {
                        self.toggle_mod_clean = Some(false);
                    } else if !down && mod_slot(code).is_some() && self.held != mods {
                        if self.toggle_mod_clean.take() == Some(true) {
                            actions.push(Action::Toggle);
                        }
                    }
                }
                if self.held == Mods::default() {
                    self.toggle_mod_clean = None; // everything released: re-arm
                }
            }
            None => {}
        }

        match b.cancel {
            Some(Binding::Combo { mods, vk }) => {
                if down && code == vk && was_held == mods && session_active {
                    swallow = true;
                    actions.push(Action::Cancel);
                }
            }
            Some(Binding::ModifierChord { mods }) => {
                if session_active && chord_completed(mods) {
                    mask_win |= mods.win;
                    actions.push(Action::Cancel);
                }
            }
            Some(Binding::ModifierOnly { vk }) => {
                if down && code == vk && session_active && !was_held_key(was_held, code) {
                    actions.push(Action::Cancel);
                }
            }
            None => {}
        }

        if down && swallow && !is_modifier(code) && !self.swallowed.contains(&code) {
            self.swallowed.push(code);
        }
        if !down {
            if let Some(i) = self.swallowed.iter().position(|c| *c == code) {
                self.swallowed.remove(i);
                swallow = true;
            }
        }
        Outcome { actions, swallow, mask_win }
    }
}

/// True if this modifier's slot was already down (auto-repeat of a held modifier).
fn was_held_key(was_held: Mods, code: u16) -> bool {
    let mut m = was_held;
    mod_slot(code).is_some_and(|slot| *slot(&mut m))
}

// ---------- runtime ----------

pub type SharedBindings = Arc<RwLock<Bindings>>;

/// Taps shorter than this on push-to-talk are treated as accidental and cancelled quietly.
const PTT_MIN_HOLD: Duration = Duration::from_millis(250);

struct Queued {
    action: Action,
    at: Instant,
}

/// Start the platform key hook and the dictation worker. Returns the shared bindings handle that
/// settings updates write to.
pub fn start(app: &AppHandle, bindings: Bindings) -> Result<SharedBindings, String> {
    let shared = Arc::new(RwLock::new(bindings));
    let (tx, rx) = mpsc::channel::<Queued>();

    // Cancel must work while the worker is blocked in a transcription, so it bypasses the queue.
    let cancel_app = app.clone();
    let send = move |action: Action| {
        if action == Action::Cancel {
            let a = cancel_app.clone();
            std::thread::spawn(move || {
                let _ = dictation::cancel(&a);
            });
        } else {
            let _ = tx.send(Queued { action, at: Instant::now() });
        }
    };

    let worker_app = app.clone();
    std::thread::Builder::new()
        .name("voxelle-hotkey-worker".into())
        .spawn(move || run_worker(worker_app, rx))
        .map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        let probe_app = app.clone();
        let session_active = move || {
            probe_app
                .state::<AppState>()
                .session_active
                .load(std::sync::atomic::Ordering::SeqCst)
        };
        windows::install(shared.clone(), Box::new(send), Box::new(session_active))?;
    }
    #[cfg(not(windows))]
    {
        let _ = send;
        return Err("Global shortcuts are not implemented on this platform yet.".into());
    }
    Ok(shared)
}

pub const CAPTURE_EVENT: &str = "shortcut_capture";
/// The recorder swallows every key, so it must never stay on if the UI goes away.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(15);
static CAPTURE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Start recording a shortcut: the next keys pressed (until all are released) are reported to the
/// UI as `CAPTURE_EVENT` updates and do nothing else. Ends by itself after `CAPTURE_TIMEOUT`.
pub fn begin_capture(app: &AppHandle) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;
    let generation = CAPTURE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    #[cfg(windows)]
    {
        let sink_app = app.clone();
        windows::begin_capture(Box::new(move |u| {
            let _ = sink_app.emit(CAPTURE_EVENT, u);
        }))?;
    }
    #[cfg(not(windows))]
    return Err("Recording shortcuts is not implemented on this platform yet.".into());
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(CAPTURE_TIMEOUT);
        if CAPTURE_GENERATION.load(Ordering::SeqCst) == generation && end_capture() {
            let _ = app.emit(
                CAPTURE_EVENT,
                capture::CaptureUpdate::Rejected { reason: "No keys were pressed in time.".into() },
            );
        }
    });
    #[allow(unreachable_code)]
    Ok(())
}

/// Stop recording. Returns true if a recording was still in progress.
pub fn end_capture() -> bool {
    #[cfg(windows)]
    return windows::end_capture();
    #[cfg(not(windows))]
    false
}

fn run_worker(app: AppHandle, rx: mpsc::Receiver<Queued>) {
    let mut last_finished = Instant::now();
    let mut ptt_started: Option<Instant> = None;
    while let Ok(q) = rx.recv() {
        let state = app.state::<AppState>().session.lock().unwrap().state();
        let startish = matches!(q.action, Action::Toggle | Action::PttPressed);
        // A press that arrived while we were busy transcribing must not start a surprise recording.
        if startish && q.at < last_finished && state != SessionState::Recording {
            continue;
        }
        let result = match q.action {
            Action::Toggle => dictation::toggle(&app),
            Action::PttPressed => {
                let r = if state == SessionState::Idle || state.is_terminal() {
                    dictation::start(&app)
                } else {
                    Ok(())
                };
                if r.is_ok() && state != SessionState::Recording {
                    ptt_started = Some(q.at);
                }
                r
            }
            Action::PttReleased => match ptt_started.take() {
                Some(t0) if state == SessionState::Recording => {
                    if q.at.duration_since(t0) < PTT_MIN_HOLD {
                        dictation::cancel(&app)
                    } else {
                        dictation::stop(&app).map(|_| ())
                    }
                }
                _ => Ok(()),
            },
            Action::PttChord => match ptt_started.take() {
                Some(_) if state == SessionState::Recording => dictation::cancel(&app),
                _ => Ok(()),
            },
            Action::Cancel => Ok(()), // handled out-of-band
        };
        if let Err(e) = result {
            if e != "cancelled" {
                eprintln!("[voxelle] hotkey {:?}: {e}", q.action);
            }
        }
        last_finished = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(toggle: &str, ptt: Option<&str>, cancel: Option<&str>) -> Bindings {
        Bindings {
            toggle: Some(parse_binding(toggle).unwrap()),
            push_to_talk: ptt.map(|p| parse_binding(p).unwrap()),
            cancel: cancel.map(|c| parse_binding(c).unwrap()),
        }
    }

    #[test]
    fn parses_common_shortcuts() {
        assert_eq!(
            parse_binding("CommandOrControl+Shift+Space").unwrap(),
            Binding::Combo { mods: Mods { ctrl: true, shift: true, ..Default::default() }, vk: vk::SPACE }
        );
        assert_eq!(parse_binding("rightctrl").unwrap(), Binding::ModifierOnly { vk: vk::RCONTROL });
        assert_eq!(parse_binding("F9").unwrap(), Binding::Combo { mods: Mods::default(), vk: vk::F1 + 8 });
        assert_eq!(
            parse_binding("Alt + d").unwrap(),
            Binding::Combo { mods: Mods { alt: true, ..Default::default() }, vk: b'D' as u16 }
        );
        assert_eq!(parse_binding("Escape").unwrap(), Binding::Combo { mods: Mods::default(), vk: vk::ESCAPE });
    }

    #[test]
    fn rejects_bad_shortcuts() {
        assert_eq!(parse_binding(""), Err(ParseError::Empty));
        assert!(matches!(parse_binding("Ctrl+Shift"), Ok(Binding::ModifierChord { .. })), "two modifiers = chord");
        assert_eq!(parse_binding("Ctrl+A+B"), Err(ParseError::NeedsOneKey));
        assert!(matches!(parse_binding("Ctrl+Enter"), Err(ParseError::Reserved(_))));
        assert!(matches!(parse_binding("Ctrl+Nope"), Err(ParseError::UnknownKey(_))));
        assert_eq!(parse_binding("Ctrl+RightCtrl"), Err(ParseError::NeedsOneKey));
    }

    #[test]
    fn same_toggle_and_ptt_is_rejected() {
        let s = crate::storage::Settings {
            toggle_hotkey: "F9".into(),
            push_to_talk_hotkey: Some("f9".into()),
            ..Default::default()
        };
        assert!(Bindings::from_settings(&s).is_err());
    }

    #[test]
    fn toggle_fires_once_with_exact_modifiers_and_is_swallowed() {
        let bind = b("Ctrl+Shift+Space", None, None);
        let mut m = Matcher::default();
        m.on_key(&bind, vk::LCONTROL, true, false);
        m.on_key(&bind, vk::LSHIFT, true, false);
        let o = m.on_key(&bind, vk::SPACE, true, false);
        assert_eq!(o.actions, vec![Action::Toggle]);
        assert!(o.swallow);
        // auto-repeat: swallowed, no second toggle
        let o = m.on_key(&bind, vk::SPACE, true, false);
        assert!(o.actions.is_empty() && o.swallow);
        // key-up of the swallowed key is swallowed too
        assert!(m.on_key(&bind, vk::SPACE, false, false).swallow);
    }

    #[test]
    fn toggle_requires_exact_modifiers() {
        let bind = b("Ctrl+Shift+Space", None, None);
        let mut m = Matcher::default();
        m.on_key(&bind, vk::LCONTROL, true, false);
        let o = m.on_key(&bind, vk::SPACE, true, false); // missing Shift
        assert!(o.actions.is_empty() && !o.swallow);
        m.on_key(&bind, vk::SPACE, false, false);
        m.on_key(&bind, vk::LSHIFT, true, false);
        m.on_key(&bind, vk::LMENU, true, false); // extra Alt
        let o = m.on_key(&bind, vk::SPACE, true, false);
        assert!(o.actions.is_empty());
    }

    #[test]
    fn modifier_only_ptt_press_and_release() {
        let bind = b("F9", Some("RightCtrl"), None);
        let mut m = Matcher::default();
        let o = m.on_key(&bind, vk::RCONTROL, true, false);
        assert_eq!(o.actions, vec![Action::PttPressed]);
        assert!(!o.swallow, "never swallow a modifier");
        assert!(m.on_key(&bind, vk::RCONTROL, true, false).actions.is_empty(), "repeat ignored");
        assert_eq!(m.on_key(&bind, vk::RCONTROL, false, false).actions, vec![Action::PttReleased]);
    }

    #[test]
    fn modifier_only_ptt_used_in_a_chord_is_cancelled() {
        let bind = b("F9", Some("RightCtrl"), None);
        let mut m = Matcher::default();
        m.on_key(&bind, vk::RCONTROL, true, false);
        let o = m.on_key(&bind, b'C' as u16, true, false);
        assert_eq!(o.actions, vec![Action::PttChord]);
        assert!(!o.swallow, "the user's Ctrl+C must still reach the app");
        assert!(m.on_key(&bind, vk::RCONTROL, false, false).actions.is_empty());
    }

    #[test]
    fn left_ctrl_does_not_trigger_right_ctrl_ptt() {
        let bind = b("F9", Some("RightCtrl"), None);
        let mut m = Matcher::default();
        assert!(m.on_key(&bind, vk::LCONTROL, true, false).actions.is_empty());
    }

    #[test]
    fn combo_ptt_releases_on_key_or_modifier_up() {
        let bind = b("F9", Some("Ctrl+Space"), None);
        let mut m = Matcher::default();
        m.on_key(&bind, vk::LCONTROL, true, false);
        let o = m.on_key(&bind, vk::SPACE, true, false);
        assert_eq!(o.actions, vec![Action::PttPressed]);
        assert!(o.swallow);
        assert_eq!(m.on_key(&bind, vk::LCONTROL, false, false).actions, vec![Action::PttReleased]);
        assert!(m.on_key(&bind, vk::SPACE, false, false).swallow, "swallowed key-up stays hidden");
    }

    #[test]
    fn modifier_only_toggle_fires_on_clean_release() {
        let bind = b("RightAlt", None, None);
        let mut m = Matcher::default();
        assert!(m.on_key(&bind, vk::RMENU, true, false).actions.is_empty());
        assert_eq!(m.on_key(&bind, vk::RMENU, false, false).actions, vec![Action::Toggle]);
        // AltGr+E (typing a character) must not toggle
        m.on_key(&bind, vk::RMENU, true, false);
        m.on_key(&bind, b'E' as u16, true, false);
        assert!(m.on_key(&bind, vk::RMENU, false, false).actions.is_empty());
    }

    #[test]
    fn cancel_only_when_session_active() {
        let bind = b("F9", None, Some("Escape"));
        let mut m = Matcher::default();
        let o = m.on_key(&bind, vk::ESCAPE, true, false);
        assert!(o.actions.is_empty() && !o.swallow, "Escape passes through when idle");
        m.on_key(&bind, vk::ESCAPE, false, false);
        let o = m.on_key(&bind, vk::ESCAPE, true, true);
        assert_eq!(o.actions, vec![Action::Cancel]);
        assert!(o.swallow);
        assert!(m.on_key(&bind, vk::ESCAPE, false, false).swallow);
    }

    #[test]
    fn parses_and_formats_modifier_chords() {
        let chord = parse_binding("win+ctrl").unwrap();
        assert_eq!(chord, Binding::ModifierChord { mods: Mods { ctrl: true, win: true, ..Default::default() } });
        assert_eq!(format_binding(&chord), "Ctrl+Win");
        assert_eq!(parse_binding("Ctrl"), Err(ParseError::NeedsOneKey), "one generic modifier is ambiguous");
        for s in ["Ctrl+Shift+Space", "RightCtrl", "Ctrl+Win", "Alt+F4", "Ctrl+Backquote", "Numpad5", "PageDown"] {
            assert_eq!(format_binding(&parse_binding(s).unwrap()), s);
        }
    }

    #[test]
    fn chord_ptt_presses_in_any_order_and_masks_start_menu() {
        let bind = b("F9", Some("Ctrl+Win"), None);
        let mut m = Matcher::default();
        assert!(m.on_key(&bind, vk::LWIN, true, false).actions.is_empty());
        let o = m.on_key(&bind, vk::LCONTROL, true, false);
        assert_eq!(o.actions, vec![Action::PttPressed]);
        assert!(o.mask_win, "releasing Win later must not open Start");
        assert!(!o.swallow, "modifiers are never swallowed");
        assert!(m.on_key(&bind, vk::LCONTROL, true, false).actions.is_empty(), "repeat ignored");
        assert!(m.on_key(&bind, vk::MASK, true, false).actions.is_empty(), "our mask key is not a chord");
        assert_eq!(m.on_key(&bind, vk::LWIN, false, false).actions, vec![Action::PttReleased]);
        assert!(m.on_key(&bind, vk::LCONTROL, false, false).actions.is_empty());
    }

    #[test]
    fn chord_ptt_used_for_a_system_shortcut_is_cancelled() {
        let bind = b("F9", Some("Ctrl+Win"), None);
        let mut m = Matcher::default();
        m.on_key(&bind, vk::LCONTROL, true, false);
        m.on_key(&bind, vk::LWIN, true, false);
        let o = m.on_key(&bind, vk::RIGHT, true, false); // Ctrl+Win+Right = switch desktop
        assert_eq!(o.actions, vec![Action::PttChord]);
        assert!(!o.swallow, "the system shortcut must still work");
    }

    #[test]
    fn chord_toggle_fires_on_clean_release_only() {
        let bind = b("Ctrl+Win", None, None);
        let mut m = Matcher::default();
        m.on_key(&bind, vk::LCONTROL, true, false);
        assert!(m.on_key(&bind, vk::LWIN, true, false).mask_win);
        assert_eq!(m.on_key(&bind, vk::LWIN, false, false).actions, vec![Action::Toggle]);
        assert!(m.on_key(&bind, vk::LCONTROL, false, false).actions.is_empty(), "fires once");
        // Ctrl+Win+D (show desktop) must not toggle.
        m.on_key(&bind, vk::LCONTROL, true, false);
        m.on_key(&bind, vk::LWIN, true, false);
        m.on_key(&bind, b'D' as u16, true, false);
        m.on_key(&bind, b'D' as u16, false, false);
        assert!(m.on_key(&bind, vk::LWIN, false, false).actions.is_empty());
        m.on_key(&bind, vk::LCONTROL, false, false);
        // Re-armed after everything was released.
        m.on_key(&bind, vk::LCONTROL, true, false);
        m.on_key(&bind, vk::LWIN, true, false);
        assert_eq!(m.on_key(&bind, vk::LCONTROL, false, false).actions, vec![Action::Toggle]);
    }

    #[test]
    fn ctrl_alone_does_not_trigger_ctrl_win_chord() {
        let bind = b("F9", Some("Ctrl+Win"), None);
        let mut m = Matcher::default();
        assert!(m.on_key(&bind, vk::LCONTROL, true, false).actions.is_empty());
        assert!(m.on_key(&bind, b'C' as u16, true, false).actions.is_empty(), "Ctrl+C is untouched");
    }
}
