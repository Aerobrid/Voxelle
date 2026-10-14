//! Session lifecycle: wraps the state machine with the live snapshot pushed to the UI.

pub mod state;

pub use state::{Event, SessionState, TransitionError};

use serde::Serialize;

/// Snapshot mirrored to the renderer (see src/ipc/types.ts `SessionSnapshot`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub state: SessionState,
    pub level: f32,
    pub message: Option<String>,
    pub elapsed_ms: u64,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            state: SessionState::Idle,
            level: 0.0,
            message: None,
            elapsed_ms: 0,
        }
    }
}

/// Owns the current session state. All state changes go through `apply`, which enforces the
/// machine's guards. Volatile fields (level/elapsed/message) reset when returning to idle or
/// starting a fresh recording.
pub struct Session {
    snapshot: SessionSnapshot,
}

impl Session {
    pub fn new() -> Self {
        Self {
            snapshot: SessionSnapshot::default(),
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot.clone()
    }

    pub fn state(&self) -> SessionState {
        self.snapshot.state
    }

    /// Apply an event. Returns the new snapshot or a TransitionError (which the caller surfaces;
    /// the state is left unchanged on error).
    pub fn apply(&mut self, event: Event) -> Result<SessionSnapshot, TransitionError> {
        let next = self.snapshot.state.next(event)?;
        self.snapshot.state = next;
        if matches!(next, SessionState::Idle | SessionState::Recording) {
            self.snapshot.message = None;
            self.snapshot.level = 0.0;
            self.snapshot.elapsed_ms = 0;
        }
        Ok(self.snapshot.clone())
    }

    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.snapshot.message = Some(msg.into());
    }

    pub fn set_level(&mut self, level: f32) {
        self.snapshot.level = level.clamp(0.0, 1.0);
    }

    pub fn set_elapsed_ms(&mut self, ms: u64) {
        self.snapshot.elapsed_ms = ms;
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_rejects_illegal_and_keeps_state() {
        let mut s = Session::new();
        assert_eq!(s.state(), SessionState::Idle);
        assert!(s.apply(Event::Stop).is_err());
        assert_eq!(
            s.state(),
            SessionState::Idle,
            "state unchanged after illegal event"
        );
    }

    #[test]
    fn volatile_fields_reset_on_new_recording() {
        let mut s = Session::new();
        s.apply(Event::Start).unwrap();
        s.set_level(0.8);
        s.set_elapsed_ms(1234);
        s.set_message("x");
        s.apply(Event::Cancel).unwrap(); // -> Cancelled
        s.apply(Event::Start).unwrap(); // -> Recording resets volatiles
        let snap = s.snapshot();
        assert_eq!(snap.level, 0.0);
        assert_eq!(snap.elapsed_ms, 0);
        assert!(snap.message.is_none());
    }
}
