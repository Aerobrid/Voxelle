# P5 - MCP server
Optional; dictation never depends on it.
Tools: search_notes, get_note, create_note, search_transcripts, get_transcript, search_vocabulary, get_word_details, save_vocabulary_word, get_speech_coach_summary.
- stdio by default. HTTP only if needed, loopback-bound, with the protections in docs/01-security.md section 5.
- Validate every input. Expose only user-approved data. Writes are explicit and approved in-app.
- Never expose recording, screenshots, shell, or arbitrary file reads.
- Document how to connect a local MCP client. Integration test for every exposed tool, including permission-denied and malicious-input cases.
