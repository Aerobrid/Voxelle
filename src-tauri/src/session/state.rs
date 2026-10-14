//! The dictation state machine. Pure logic, no I/O — the backbone of P0 §0.1.
//! Illegal transitions are rejected; there is never more than one active session.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Idle,
    Recording,
    Processing,
    Inserting,
    Completed,
    Cancelled,
    Failed,
}

/// Inputs that drive the machine. Produced by the hotkey/UI layer and by the audio/asr/insert
/// workers reporting results or errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Start,
    Stop,
    TranscriptReady,
    Inserted,
    CopiedOnly,
    Cancel,
    DeviceError,
    AsrError,
    InsertError,
    Reset,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("illegal transition: {event:?} is not allowed in state {state:?}")]
pub struct TransitionError {
    pub state: SessionState,
    pub event: Event,
}

impl SessionState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            SessionState::Completed | SessionState::Cancelled | SessionState::Failed
        )
    }

    /// Entering any of these releases the microphone (enforced by the session manager).
    pub fn mic_should_be_open(self) -> bool {
        matches!(self, SessionState::Recording)
    }

    /// Compute the next state, rejecting illegal transitions.
    pub fn next(self, event: Event) -> Result<SessionState, TransitionError> {
        use Event::*;
        use SessionState::*;
        let next = match (self, event) {
            // A new session may start only from idle or a terminal state — never while one is
            // already recording/processing/inserting (no duplicate sessions).
            (Idle, Start) | (Completed, Start) | (Cancelled, Start) | (Failed, Start) => Recording,

            (Recording, Stop) => Processing,
            (Recording, Cancel) => Cancelled,
            (Recording, DeviceError) => Failed,

            (Processing, TranscriptReady) => Inserting,
            (Processing, Cancel) => Cancelled,
            (Processing, AsrError) => Failed,

            // Both inserted and copied-only complete the session; the InsertResult records which.
            (Inserting, Inserted) | (Inserting, CopiedOnly) => Completed,
            (Inserting, InsertError) => Failed,

            (Completed, Reset) | (Cancelled, Reset) | (Failed, Reset) => Idle,

            _ => return Err(TransitionError { state: self, event }),
        };
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_inserted() {
        let s = SessionState::Idle;
        let s = s.next(Event::Start).unwrap();
        assert_eq!(s, SessionState::Recording);
        let s = s.next(Event::Stop).unwrap();
        assert_eq!(s, SessionState::Processing);
        let s = s.next(Event::TranscriptReady).unwrap();
        assert_eq!(s, SessionState::Inserting);
        let s = s.next(Event::Inserted).unwrap();
        assert_eq!(s, SessionState::Completed);
        assert!(s.is_terminal());
    }

    #[test]
    fn copied_only_also_completes() {
        let s = SessionState::Inserting.next(Event::CopiedOnly).unwrap();
        assert_eq!(s, SessionState::Completed);
    }

    #[test]
    fn cancel_from_recording_and_processing() {
        assert_eq!(
            SessionState::Recording.next(Event::Cancel).unwrap(),
            SessionState::Cancelled
        );
        assert_eq!(
            SessionState::Processing.next(Event::Cancel).unwrap(),
            SessionState::Cancelled
        );
    }

    #[test]
    fn errors_route_to_failed() {
        assert_eq!(
            SessionState::Recording.next(Event::DeviceError).unwrap(),
            SessionState::Failed
        );
        assert_eq!(
            SessionState::Processing.next(Event::AsrError).unwrap(),
            SessionState::Failed
        );
        assert_eq!(
            SessionState::Inserting.next(Event::InsertError).unwrap(),
            SessionState::Failed
        );
    }

    #[test]
    fn no_duplicate_session() {
        // Starting while already active is illegal.
        assert!(SessionState::Recording.next(Event::Start).is_err());
        assert!(SessionState::Processing.next(Event::Start).is_err());
        assert!(SessionState::Inserting.next(Event::Start).is_err());
    }

    #[test]
    fn can_restart_from_terminal() {
        for t in [
            SessionState::Completed,
            SessionState::Cancelled,
            SessionState::Failed,
        ] {
            assert_eq!(t.next(Event::Start).unwrap(), SessionState::Recording);
            assert_eq!(t.next(Event::Reset).unwrap(), SessionState::Idle);
        }
    }

    #[test]
    fn illegal_transitions_rejected() {
        // Stop only valid while recording.
        assert!(SessionState::Idle.next(Event::Stop).is_err());
        assert!(SessionState::Processing.next(Event::Stop).is_err());
        // Cancel not valid once inserting or idle.
        assert!(SessionState::Idle.next(Event::Cancel).is_err());
        assert!(SessionState::Inserting.next(Event::Cancel).is_err());
        // TranscriptReady only valid while processing.
        assert!(SessionState::Recording
            .next(Event::TranscriptReady)
            .is_err());
    }

    #[test]
    fn mic_open_only_while_recording() {
        assert!(SessionState::Recording.mic_should_be_open());
        for s in [
            SessionState::Idle,
            SessionState::Processing,
            SessionState::Inserting,
            SessionState::Completed,
            SessionState::Cancelled,
            SessionState::Failed,
        ] {
            assert!(!s.mic_should_be_open());
        }
    }
}
