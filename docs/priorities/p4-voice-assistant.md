# P4 - Voice AI assistant and agents
Optional, local by default, hotkey-triggered, no wake word.

## Interaction
General questions; summarize, explain, or rewrite selected text; generate content at the cursor; work with explicitly selected notes or transcripts; screenshot context only after an explicit user action. Floating response panel with copy and insert-at-cursor. In-place edits keep the original recoverable. If no editable field is focused, show the response in the panel and do not pretend to insert it.

## Tools
Narrow, modular interface: search approved notes, retrieve selected transcripts, search vocabulary DB, summarize a user-selected file, save a note, search user-approved folders. No shell, no arbitrary filesystem access. Allowlists, input validation, scoped permissions, local audit log. Approval required for destructive actions, file writes, external requests, and sensitive data. Implement exactly as docs/01-security.md specifies; this priority is not done until the section 7 adversarial tests pass.

## Providers
Default local. Remote providers are an advanced, manual, opt-in feature: no required API keys, each provider marked local/remote, disclosure before any content is sent, never a silent fallback. Do not hard-code one model.
