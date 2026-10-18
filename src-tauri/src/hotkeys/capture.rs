//! Shortcut recorder: turns the physical keys the user presses into a binding string. Pure (fed
//! virtual-key events), so it is unit tested; the platform hook feeds it while the Settings page
//! is recording, and swallows those keys so they do nothing else (e.g. Win does not open Start).
//!
//! The shortcut is the largest set of keys held at once, finalized when every key is released.

use super::{check_allowed_with, LayoutHotkeys, format_binding, is_modifier, key_name, parse_binding, reserved_name, vk, Role};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CaptureUpdate {
    /// Keys currently held, for live display (e.g. "Ctrl+Win").
    Progress { keys: String },
    /// All keys released: the canonical shortcut text, ready to save.
    Done { shortcut: String },
    /// All keys released but the combination cannot be a shortcut (shown to the user).
    Rejected { reason: String },
    /// Nothing was pressed for a while: recording stopped quietly, the shortcut is unchanged.
    Ended,
    /// Escape pressed on its own (and Escape can't be this shortcut): the user backed out.
    Dismissed,
}

#[derive(Debug)]
pub struct Capture {
    role: Role,
    layouts: LayoutHotkeys,
    held: Vec<u16>,
    peak: Vec<u16>,
}

/// Generic name for a modifier key (either side), used when several keys are combined.
fn generic_modifier(code: u16) -> &'static str {
    match code {
        vk::CONTROL | vk::LCONTROL | vk::RCONTROL => "Ctrl",
        vk::SHIFT | vk::LSHIFT | vk::RSHIFT => "Shift",
        vk::MENU | vk::LMENU | vk::RMENU => "Alt",
        _ => "Win",
    }
}

/// Canonical shortcut text for a set of held keys, or why it cannot be one.
pub fn shortcut_for(keys: &[u16]) -> Result<String, String> {
    if let Some(name) = keys.iter().find_map(|k| reserved_name(*k)) {
        return Err(format!("{name} can't be part of a shortcut (it would submit or move focus in other apps)."));
    }
    let (mods, others): (Vec<u16>, Vec<u16>) = keys.iter().partition(|k| is_modifier(**k));
    let text = match (mods.as_slice(), others.as_slice()) {
        ([], []) => return Err("No keys were pressed.".into()),
        // One modifier alone keeps its side, so plain Ctrl still works for typing shortcuts.
        ([m], []) => key_name(*m).ok_or("That key can't be used.")?,
        (_, []) | (_, [_]) => {
            let mut parts: Vec<String> = mods.iter().map(|m| generic_modifier(*m).to_string()).collect();
            if let [k] = others.as_slice() {
                parts.push(key_name(*k).ok_or("That key can't be used in a shortcut.")?);
            }
            parts.join("+")
        }
        _ => return Err("Use one key plus modifiers (like Ctrl+Shift+Space), or only modifiers (like Ctrl+Win).".into()),
    };
    parse_binding(&text)
        .map(|b| format_binding(&b))
        .map_err(|_| "Hold two different modifiers (like Ctrl+Win), or use one side-specific key such as RightCtrl.".to_string())
}

impl Capture {
    /// Conservative: assumes Windows uses Ctrl+Shift and Alt+Shift for layout switching.
    pub fn new(role: Role) -> Self {
        Self::with_layouts(role, LayoutHotkeys::ALL)
    }

    /// `layouts` is read once up front: the recorder runs inside the keyboard hook, which must not
    /// query the system.
    pub fn with_layouts(role: Role, layouts: LayoutHotkeys) -> Self {
        Self { role, layouts, held: Vec::new(), peak: Vec::new() }
    }

    /// Keys whose key-down the recorder took. Only their key-ups belong to it: swallowing the
    /// key-up of a key pressed before recording started would leave it stuck down in Windows.
    pub fn held(&self) -> &[u16] {
        &self.held
    }

    pub fn on_key(&mut self, code: u16, down: bool) -> Option<CaptureUpdate> {
        if code == vk::MASK || code == 0 || code == 0xFF {
            return None;
        }
        if down {
            if self.held.contains(&code) {
                return None; // auto-repeat
            }
            self.held.push(code);
            if self.held.len() >= self.peak.len() {
                self.peak = self.held.clone();
            }
            let keys = shortcut_for(&self.held).unwrap_or_else(|_| {
                self.held.iter().map(|k| key_name(*k).unwrap_or_else(|| "?".into())).collect::<Vec<_>>().join("+")
            });
            return Some(CaptureUpdate::Progress { keys });
        }
        let before = self.held.len();
        self.held.retain(|k| *k != code);
        if self.held.is_empty() && before > 0 {
            let peak = std::mem::take(&mut self.peak);
            if peak == [vk::ESCAPE] && self.role != Role::Cancel {
                return Some(CaptureUpdate::Dismissed);
            }
            let checked = shortcut_for(&peak).and_then(|s| {
                let binding = parse_binding(&s).map_err(|e| e.to_string())?;
                check_allowed_with(&binding, self.role, self.layouts).map(|_| s)
            });
            return Some(match checked {
                Ok(shortcut) => CaptureUpdate::Done { shortcut },
                Err(reason) => CaptureUpdate::Rejected { reason },
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(c: &mut Capture, keys: &[u16]) -> Vec<CaptureUpdate> {
        let mut out = Vec::new();
        for k in keys {
            out.extend(c.on_key(*k, true));
        }
        for k in keys {
            out.extend(c.on_key(*k, false));
        }
        out
    }

    fn last(updates: Vec<CaptureUpdate>) -> CaptureUpdate {
        updates.into_iter().last().unwrap()
    }

    #[test]
    fn records_ctrl_win_as_a_chord() {
        let mut c = Capture::new(Role::Toggle);
        let out = press(&mut c, &[vk::LCONTROL, vk::LWIN]);
        assert_eq!(out[1], CaptureUpdate::Progress { keys: "Ctrl+Win".into() });
        assert_eq!(last(out), CaptureUpdate::Done { shortcut: "Ctrl+Win".into() });
    }

    #[test]
    fn records_combo_in_canonical_order() {
        let mut c = Capture::new(Role::Toggle);
        let out = press(&mut c, &[vk::LSHIFT, vk::RCONTROL, vk::SPACE]);
        assert_eq!(last(out), CaptureUpdate::Done { shortcut: "Ctrl+Shift+Space".into() });
    }

    #[test]
    fn single_modifier_keeps_its_side() {
        let mut c = Capture::new(Role::Toggle);
        assert_eq!(last(press(&mut c, &[vk::RCONTROL])), CaptureUpdate::Done { shortcut: "RightCtrl".into() });
        assert_eq!(last(press(&mut c, &[vk::RMENU])), CaptureUpdate::Done { shortcut: "RightAlt".into() });
    }

    #[test]
    fn single_key_and_function_keys() {
        let mut c = Capture::new(Role::Toggle);
        assert_eq!(last(press(&mut c, &[vk::F1 + 8])), CaptureUpdate::Done { shortcut: "F9".into() });
        assert_eq!(
            last(press(&mut c, &[vk::LCONTROL, vk::LMENU, b'D' as u16])),
            CaptureUpdate::Done { shortcut: "Ctrl+Alt+D".into() }
        );
    }

    #[test]
    fn keys_that_would_break_typing_are_refused() {
        let mut c = Capture::new(Role::Toggle);
        for keys in [&[b'A' as u16][..], &[vk::LEFT], &[vk::SPACE], &[vk::LWIN], &[vk::LMENU, b'D' as u16], &[vk::LCONTROL, vk::LMENU]] {
            assert!(matches!(last(press(&mut c, keys)), CaptureUpdate::Rejected { .. }), "{keys:?}");
        }
    }

    #[test]
    fn cancel_may_use_escape_but_toggle_may_not() {
        let mut toggle = Capture::new(Role::Toggle);
        assert_eq!(last(press(&mut toggle, &[vk::ESCAPE])), CaptureUpdate::Dismissed, "Esc backs out");
        assert!(matches!(last(press(&mut toggle, &[vk::LCONTROL, vk::ESCAPE])), CaptureUpdate::Rejected { .. }));
        let mut cancel = Capture::new(Role::Cancel);
        assert_eq!(last(press(&mut cancel, &[vk::ESCAPE])), CaptureUpdate::Done { shortcut: "Escape".into() });
    }

    #[test]
    fn ctrl_shift_records_when_windows_does_not_use_it() {
        let mut c = Capture::with_layouts(Role::PushToTalk, LayoutHotkeys::NONE);
        assert_eq!(last(press(&mut c, &[vk::LCONTROL, vk::LSHIFT])), CaptureUpdate::Done { shortcut: "Ctrl+Shift".into() });
        let mut c = Capture::new(Role::PushToTalk);
        assert!(matches!(last(press(&mut c, &[vk::LCONTROL, vk::LSHIFT])), CaptureUpdate::Rejected { .. }));
    }

    #[test]
    fn held_lists_only_keys_pressed_while_recording() {
        let mut c = Capture::new(Role::Toggle);
        assert!(c.on_key(vk::LCONTROL, false).is_none()); // pressed before recording began
        assert!(c.held().is_empty());
        c.on_key(vk::LWIN, true);
        assert_eq!(c.held(), &[vk::LWIN]);
    }

    #[test]
    fn uses_the_largest_set_held_at_once() {
        let mut c = Capture::new(Role::Toggle);
        c.on_key(vk::LCONTROL, true);
        c.on_key(vk::LWIN, true);
        c.on_key(vk::LWIN, false); // released first: still Ctrl+Win
        let out = c.on_key(vk::LCONTROL, false);
        assert_eq!(out, Some(CaptureUpdate::Done { shortcut: "Ctrl+Win".into() }));
    }

    #[test]
    fn auto_repeat_and_mask_key_are_ignored() {
        let mut c = Capture::new(Role::Toggle);
        assert!(c.on_key(vk::LCONTROL, true).is_some());
        assert!(c.on_key(vk::LCONTROL, true).is_none());
        assert!(c.on_key(vk::MASK, true).is_none());
    }

    #[test]
    fn rejects_reserved_and_two_plain_keys() {
        let mut c = Capture::new(Role::Toggle);
        assert!(matches!(last(press(&mut c, &[vk::LCONTROL, vk::RETURN])), CaptureUpdate::Rejected { .. }));
        assert!(matches!(last(press(&mut c, &[b'A' as u16, b'B' as u16])), CaptureUpdate::Rejected { .. }));
        // Both Ctrl keys = one generic modifier: ambiguous, rejected with guidance.
        assert!(matches!(last(press(&mut c, &[vk::LCONTROL, vk::RCONTROL])), CaptureUpdate::Rejected { .. }));
    }

    #[test]
    fn every_recorded_shortcut_parses_back() {
        for keys in [&[vk::LCONTROL, vk::LWIN][..], &[vk::RMENU], &[vk::LCONTROL, vk::LMENU, vk::OEM_3], &[vk::DELETE]] {
            let s = shortcut_for(keys).unwrap();
            assert_eq!(format_binding(&parse_binding(&s).unwrap()), s);
        }
    }
}
