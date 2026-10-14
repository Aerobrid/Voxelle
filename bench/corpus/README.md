# ASR evaluation corpus

Small, repeatable corpus for P0 measurements (real-time factor, word error rate).

- `sentences.tsv` — 12 original dictation-style sentences written for this project (no third-party text).
- `make-corpus.ps1` — renders each sentence to `audio/<id>.wav` (16 kHz, mono, 16-bit) with the
  offline Windows SAPI text-to-speech voice. Audio is generated locally and git-ignored.

**Limitation:** synthetic TTS speech is cleaner than a real voice in a real room, so WER here is a
lower bound. Real-microphone results are recorded separately in the P0 measurement notes.
