# P1 - Local AI text editing and dictation cleanup

## Provider abstraction
Interface for local LLMs. Start with Ollama (configurable endpoint and model name) but never require it. Leave room for an embedded llama.cpp runtime. Provider is labeled local or remote.

## Actions
Correct transcription mistakes; fix grammar and punctuation; remove fillers; improve paragraphs; make concise; professional or casual rewrite; edit selected text in place; undo/restore original.

## Rules
- Never run an LLM per keystroke. Explicit modes: raw transcription (default), lightweight cleanup, full AI refinement, user-triggered rewrite.
- The default dictation path stays fast. Cleanup runs asynchronously and is cancellable.
- The original transcript is always preserved; the AI version is stored separately. Never silently replace the original.
- Preserve meaning. Do not invent facts, change names, or turn technical terms into unrelated words. When uncertain, keep the original text.
- Custom system prompts and writing preferences, stored locally.
- Calls follow docs/01-security.md section 1 (no tools).

## Definition of done
Cleanup works against a local model, is cancellable, preserves originals, and tests prove the original survives success, failure, and cancel. Report measured added latency.
