//! Global hotkeys (P0 §0.1): toggle, push-to-talk (needs key-release, so a low-level key hook
//! rather than a press-only shortcut API), and cancel. Parsing and matching are pure and unit
//! tested; `windows.rs` feeds real key events in. Key codes are Windows virtual-key codes, which
//! other platform adapters translate into.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// Modifiers + one non-modifier key, e.g. Ctrl+Shift+Space. Matches exact modifiers.
    Combo { mods: Mods, vk: u16 },
    /// A single side-specific modifier held on its own, e.g. RightCtrl (push-to-talk friendly).
    ModifierOnly { vk: u16 },
}

pub mod vk {
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;
    pub const PAUSE: u16 = 0x13;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const INSERT: u16 = 0x2D;
    pub const LWIN: u16 = 0x5B;
    pub const RWIN: u16 = 0x5C;
    pub const F1: u16 = 0x70;
    pub const LSHIFT: u16 = 0xA0;
    pub const RSHIFT: u16 = 0xA1;
    pub const LCONTROL: u16 = 0xA2;
    pub const RCONTROL: u16 = 0xA3;
    pub const LMENU: u16 = 0xA4;
    pub const RMENU: u16 = 0xA5;
    pub const OEM_3: u16 = 0xC0; // ` ~
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
    #[error("a shortcut needs exactly one non-modifier key (or a single side-specific modifier like RightCtrl)")]
    NeedsOneKey,
    #[error("'{0}' cannot be used as a shortcut key")]
    Reserved(String),
}

fn named_key(name: &str) -> Option<u16> {
    let n = name.to_ascii_lowercase();
    let code = match n.as_str() {
        "space" => vk::SPACE,
        "esc" | "escape" => vk::ESCAPE,
        "insert" => vk::INSERT,
        "pause" => vk::PAUSE,
        "backquote" | "`" => vk::OEM_3,
        "leftctrl" | "lctrl" => vk::LCONTROL,
        "rightctrl" | "rctrl" => vk::RCONTROL,
        "leftshift" | "lshift" => vk::LSHIFT,
        "rightshift" | "rshift" => vk::RSHIFT,
        "leftalt" | "lalt" => vk::LMENU,
        "rightalt" | "ralt" | "altgr" => vk::RMENU,
        "leftwin" | "lwin" => vk::LWIN,
        "rightwin" | "rwin" => vk::RWIN,
        "enter" | "return" | "tab" | "backspace" => return None,
        _ => {
            let b = n.as_bytes();
            if b.len() == 1 && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit()) {
                return Some(b[0].to_ascii_uppercase() as u16);
            }
            if let Some(num) = n.strip_prefix('f').and_then(|d| d.parse::<u16>().ok()) {
                if (1..=24).contains(&num) {
                    return Some(vk::F1 + num - 1);
                }
            }
            return None;
        }
    };
    Some(code)
}

/// Parse "Ctrl+Shift+Space", "CommandOrControl+Shift+Space", "RightCtrl", "F9" (case-insensitive).
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
            "enter" | "return" | "tab" | "backspace" => return Err(ParseError::Reserved((*p).into())),
            _ => keys.push(named_key(p).ok_or_else(|| ParseError::UnknownKey((*p).into()))?),
        }
    }
    match keys.as_slice() {
        [k] if is_modifier(*k) && mods == Mods::default() => Ok(Binding::ModifierOnly { vk: *k }),
        [k] if !is_modifier(*k) => Ok(Binding::Combo { mods, vk: *k }),
        _ => Err(ParseError::NeedsOneKey),
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
    /// Modifier-only toggle key is down and no other key was pressed since (fires on release).
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
}

impl Matcher {
    /// Feed one physical key event. `session_active` = recording or processing (cancel applies).
    pub fn on_key(&mut self, b: &Bindings, code: u16, down: bool, session_active: bool) -> Outcome {
        let mut actions = Vec::new();
        let mut swallow = false;
        let was_held = self.held;
        if let Some(slot) = mod_slot(code) {
            *slot(&mut self.held) = down;
        }

        match b.push_to_talk {
            Some(Binding::ModifierOnly { vk }) => {
                if code == vk {
                    if down && !self.ptt_down {
                        self.ptt_down = true;
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
            Some(Binding::Combo { mods, vk }) => {
                if down && code == vk && was_held == mods && !self.ptt_down {
                    self.ptt_down = true;
                    swallow = true;
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

        if let Some(Binding::Combo { mods, vk }) = b.toggle {
            if down && code == vk && was_held == mods {
                swallow = true;
                if !self.swallowed.contains(&code) {
                    actions.push(Action::Toggle); // ignore auto-repeat
                }
            }
        }
        if let Some(Binding::ModifierOnly { vk }) = b.toggle {
            // Fire on release, and only if no other key was pressed meanwhile (not part of a chord).
            if code == vk {
                if down && self.toggle_mod_clean.is_none() {
                    self.toggle_mod_clean = Some(true);
                } else if !down && self.toggle_mod_clean.take() == Some(true) {
                    actions.push(Action::Toggle);
                }
            } else if down && self.toggle_mod_clean.is_some() {
                self.toggle_mod_clean = Some(false);
            }
        }

        if let Some(Binding::Combo { mods, vk }) = b.cancel {
            if down && code == vk && was_held == mods && session_active {
                swallow = true;
                actions.push(Action::Cancel);
            }
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
        Outcome { actions, swallow }
    }
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
        assert_eq!(parse_binding("Ctrl+Shift"), Err(ParseError::NeedsOneKey));
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
}
