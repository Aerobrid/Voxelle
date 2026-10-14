# P2 - Dictation translation

Separate hotkey/command from ordinary dictation, with its own settings.
- Explicit source and target language; optional auto-detect of the source.
- Local speech recognition. Local translation via a compatible translation model, or optionally a local LLM.
- Preserve names, code identifiers, URLs, and formatting where possible.
- If the needed model for a language pair is not installed, show which model is needed and let the user cancel. Never fall back silently to a cloud service.
- Insert the result at the cursor using the P0 insertion adapter.

Definition of done: speak language A, get language B at the cursor, offline, for at least one tested language pair; missing-model state is tested.
