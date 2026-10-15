# Manual test procedures

Automated tests live with the code; these are the human-in-the-loop checks per OS. Record the date,
OS/session, and actual result. "N/A yet" = feature not implemented. Never mark pass without observing.

## Windows (primary)
### P0 dictation end-to-end
1. Launch Voxelle. Note startup time to interactive window.
2. Open Model Manager. (In-app download is planned; for now place a catalog file in the model
   folder shown there.) Confirm the model is "Installed", press "Verify checksum" → matches.
   After one dictation, "Measured here: N× real time" appears.
3. Select a microphone; confirm the level meter responds to speech.
4. Focus a plain text editor (e.g., Notepad). Press the toggle hotkey (default Ctrl+Shift+Space);
   confirm the bottom-centre recording pill appears and Notepad keeps focus (caret still blinking). Speak one sentence. Press hotkey again to stop.
5. Confirm: state goes recording→processing→inserting→completed, and the transcript is inserted at
   the cursor. Measure stop→transcript latency.
6. Edit the inserted text; confirm no auto-Enter was sent (cursor stays on same line + no submit).
7. History: the raw transcript appears with a timestamp; copy/edit/export/delete work; edited text is
   stored separately from the raw transcript.
8. **Offline check:** disable networking (airplane mode / disable adapters); repeat steps 4–6.
   Everything must work. Confirm the privacy dashboard shows Local for the whole loop.
9. **Elevated target:** focus an app running as admin; confirm insertion reports **copied-only**
   (not a false "inserted").
10. **Model missing:** remove/rename the model; confirm a clear error state, no crash.
11. Push-to-talk: hold the PTT key (default RightCtrl), speak, release; confirm record starts on press and stops on
    release.
12. Cancel: start recording, press cancel; confirm return to idle and mic released (check Windows
    mic-in-use indicator clears).

## Linux X11
- As above; expect native paste via XTEST. Record results.

## Linux Wayland
- Global shortcut: expect **portal** or compositor route; if unavailable, confirm the app clearly
  reports global PTT as platform-limited and offers the tray/in-app toggle. Insertion may be
  copied-only — confirm it says so.

## macOS
- First launch: confirm prompts/links for Microphone, Accessibility, and (for PTT) Input Monitoring.
  Deny one and confirm a clear, non-crashing error that explains how to grant it. Then grant and
  repeat the end-to-end flow.

## Results log
| Date | OS / session | Scenario | Result |
|---|---|---|---|
| 2026-10-13 | Windows / s1 | all P0 scenarios | N/A yet — core not compiled (MSVC Build Tools pending) |
| 2026-10-14 | Windows / s2 | App launch (Vulkan dev build) | PASS — window + "Core connected"; overlay window created hidden with WS_EX_NOACTIVATE + click-through; DB created in %APPDATA%\com.voxelle.app |
| 2026-10-14 | Windows / s2 | Model preload | PASS — first run 36 s (one-time SHA-256 of 574 MB in a debug build), restart 2.2 s (verification persisted) |
| 2026-10-14 | Windows / s2 | Insertion probe (`scripts/insert-probe.ps1`, harness text box, 4 cases × 5) | 19/20 exact. 1 paste reported "inserted" but text did not arrive (false positive; see feature-status). User clipboard restored every run |
| 2026-10-14 | Windows / s2 | Mic probe (`examples/mic_probe.rs`, BT headset "Hesh ANC") | Mic start 696 ms; capture 3.0 s @16 kHz; mic released after stop. Room silent (-65 dBFS) → speech gate would reject |
| 2026-10-14 | Windows / s2 | Steps 4–12 with a real voice | NOT RUN — needs a person speaking + hotkeys (synthetic keys are ignored by design) |
| 2026-10-15 | Windows / s3 | User run, steps 4–5 (real voice, toggle hotkey) | PASS once: 5.1 s audio, 1.87 s transcribe (GPU, turbo), stop→text 1.92 s, "via clipboard-copy". First 1–3 s of speech missing (BT headset). Then **FAIL**: RPC_E_CHANGED_MODE on next start; hotkeys appeared dead (every start failed) |
| 2026-10-15 | Windows / s3 | `capture_repeat` probe after fix (20 rounds, 2 runs) + real app 6× start/stop via IPC | PASS — 0 start failures (was failing at round 3); full-length audio (was missing ~0.6 s/rec). Keep-ready: next start 0 ms, idle close after timeout, cancel releases mic |
| 2026-10-15 | Windows / s3 | Copy via real app IPC + ignored clipboard tests | PASS — text read back from another process; deliberate copy carries no history-exclusion marker |
| 2026-10-15 | Windows / s3 | User dictations after the capture fix (4 in a row, toggle + PTT shortcuts) | PASS — 4 consecutive dictations stored in History at 2:48–2:49 PM, no mic failure (observed in the app's History; user-side insertion not separately confirmed) |
| 2026-10-15 | Windows / s3 | UI v2 window chrome via real buttons (CDP) | PASS — maximize ↔ restore, close hides to tray (process alive), 2nd launch shows it again |
