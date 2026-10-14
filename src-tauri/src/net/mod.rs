//! The single network egress point (principles "Offline mode", security.md §2). Deny-by-default:
//! offline mode blocks everything; otherwise the host must be explicitly allowlisted. The P0
//! dictation loop never calls this — only the model manager will, for consented downloads.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NetError {
    #[error("offline mode is enabled; network egress blocked")]
    OfflineBlocked,
    #[error("host not allowlisted: {0}")]
    NotAllowlisted(String),
}

/// Gate every outbound request. Returns Ok only when online mode is on AND the host is allowlisted.
pub fn guard_egress(offline_mode: bool, host: &str, allowlist: &[&str]) -> Result<(), NetError> {
    if offline_mode {
        return Err(NetError::OfflineBlocked);
    }
    if !allowlist.contains(&host) {
        return Err(NetError::NotAllowlisted(host.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_blocks_even_allowlisted_hosts() {
        assert_eq!(
            guard_egress(true, "huggingface.co", &["huggingface.co"]),
            Err(NetError::OfflineBlocked)
        );
    }

    #[test]
    fn online_requires_allowlist() {
        assert_eq!(
            guard_egress(false, "evil.test", &["huggingface.co"]),
            Err(NetError::NotAllowlisted("evil.test".into()))
        );
        assert!(guard_egress(false, "huggingface.co", &["huggingface.co"]).is_ok());
    }
}
