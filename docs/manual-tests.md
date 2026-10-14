# Manual test procedures

Automated tests live with the code; these are the human-in-the-loop checks per OS. Record the date,
OS/session, and actual result. "N/A yet" = feature not implemented. Never mark pass without observing.

## Windows (primary)
### P0 dictation end-to-end
1. Launch Voxelle. Note startup time to interactive window.
2. Open Model Manager → install a whisper model (consent dialog shown, SHA-256 verified). Confirm
   measured speed appears for this machine.
3. Select a microphone; confirm the level meter responds to speech.
4. Focus a plain text editor (e.g., Notepad). Press the toggle hotkey; confirm recording indicator +
   mic-active chip. Speak one sentence. Press hotkey again to stop.
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
11. Push-to-talk: hold the PTT key, speak, release; confirm record starts on press and stops on
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
