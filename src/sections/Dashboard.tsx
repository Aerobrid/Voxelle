import { useEffect, useState } from "react";
import { core } from "../ipc/bridge";
import { canCancel, canStart, canStop } from "../ipc/transitions";
import type { DictationResult } from "../ipc/types";
import { useAppStore } from "../state/store";
import { errorText, useCore } from "../state/useCore";

const OUTCOME_TEXT: Record<string, string> = {
  inserted: "Inserted at the cursor",
  "copied-only": "Copied to the clipboard (not inserted)",
  failed: "Not inserted",
};

/** First-audio delay worth telling the user about (speech before it is lost). */
const SLOW_MIC_MS = 400;

function fmtMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)} s` : `${ms} ms`;
}

function LastResult({ result }: { result: DictationResult }) {
  const [text, setText] = useState(result.transcript);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    setText(result.transcript);
    setNote(null);
  }, [result]);

  const edited = text !== result.transcript;
  const copy = () =>
    core.copyText(text).then(
      () => setNote("Copied."),
      (e) => setNote(errorText(e)),
    );
  const saveEdit = () => {
    if (!result.historyId) return;
    core.setHistoryEdit(result.historyId, text).then(
      () => setNote("Correction saved to History (the original transcript is kept)."),
      (e) => setNote(errorText(e)),
    );
  };
  const m = result.metrics;

  return (
    <div className="card">
      <div className="row-between">
        <strong>Last dictation</strong>
        <span className={`outcome outcome--${result.insert.outcome}`}>
          {OUTCOME_TEXT[result.insert.outcome]}
        </span>
      </div>
      {result.insert.message && <p className="muted">{result.insert.message}</p>}
      <textarea
        className="transcript-edit"
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={3}
        aria-label="Transcript (editable)"
        spellCheck
      />
      <div className="actions">
        <button className="btn" onClick={copy}>
          Copy{edited ? " corrected text" : ""}
        </button>
        <button className="btn" onClick={saveEdit} disabled={!edited || !result.historyId}>
          Save correction
        </button>
        {edited && (
          <button className="btn btn--ghost" onClick={() => setText(result.transcript)}>
            Revert
          </button>
        )}
        {note && <span className="muted">{note}</span>}
      </div>
      <p className="muted small">
        {fmtMs(m.audioMs)} of audio · transcribed in {fmtMs(m.transcribeMs)} on{" "}
        {m.gpu ? "GPU" : "CPU"} (RTF {m.realTimeFactor.toFixed(2)}) · stop → text{" "}
        {fmtMs(m.stopToTranscriptMs)} · mic start{" "}
        {m.micWasReady ? "instant (kept ready)" : fmtMs(m.startLatencyMs)}
        {m.firstAudioMs != null && m.firstAudioMs > 100 ? ` · first audio after ${fmtMs(m.firstAudioMs)}` : ""}
        {m.modelLoadMs > 50 ? ` · model load ${fmtMs(m.modelLoadMs)}` : ""} · via{" "}
        {result.insert.method}
      </p>
      {!m.micWasReady && m.firstAudioMs != null && m.firstAudioMs >= SLOW_MIC_MS && (
        <p className="notice small">
          Your microphone took {fmtMs(m.firstAudioMs)} to start sending audio, so the first words may
          be missing. This is common with Bluetooth headsets. Turn on <em>Keep microphone ready</em> in
          Settings to start instantly next time.
        </p>
      )}
    </div>
  );
}

export function Dashboard() {
  const snapshot = useAppStore((s) => s.snapshot);
  const coreConnected = useAppStore((s) => s.coreConnected);
  const lastResult = useAppStore((s) => s.lastResult);
  const setSection = useAppStore((s) => s.setSection);
  const [settingsQ] = useCore(() => core.getSettings(), [snapshot.state === "idle"]);
  const [modelsQ] = useCore(() => core.listModels(), []);
  const [error, setError] = useState<string | null>(null);

  const run = (p: Promise<unknown>) => {
    setError(null);
    p.catch((e) => {
      const msg = errorText(e);
      if (msg !== "cancelled") setError(msg);
    });
  };

  const selected = modelsQ.status === "ok" ? modelsQ.data.find((m) => m.selected) : undefined;
  const anyInstalled = modelsQ.status === "ok" && modelsQ.data.some((m) => m.installed);
  const s = snapshot.state;

  return (
    <div className="main">
      <h1>Dashboard</h1>
      <p className="sub">Local, offline dictation. Nothing leaves this device.</p>

      {coreConnected && modelsQ.status === "ok" && !selected && (
        <div className="notice">
          {anyInstalled ? "No speech model selected. " : "No speech model installed yet. "}
          <button className="link" onClick={() => setSection("models")}>
            Open Model Manager
          </button>
        </div>
      )}

      <div className="card dictate">
        <div className="dictate__controls">
          {canStop(s) ? (
            <button className="btn btn--record" onClick={() => run(core.stopDictation())}>
              Stop
            </button>
          ) : (
            <button
              className="btn btn--primary"
              disabled={!coreConnected || !canStart(s)}
              onClick={() => run(core.startDictation())}
            >
              Start dictation
            </button>
          )}
          <button
            className="btn btn--ghost"
            disabled={!canCancel(s)}
            onClick={() => run(core.cancelDictation())}
          >
            Cancel
          </button>
          <span className="mono muted">{s}</span>
          {s === "recording" && (
            <>
              <span className="level level--wide" title="Input level">
                <span className="level__fill" style={{ width: `${Math.round(snapshot.level * 100)}%` }} />
              </span>
              <span className="mono">{(snapshot.elapsedMs / 1000).toFixed(1)} s</span>
            </>
          )}
        </div>
        {snapshot.message && <p className="muted">{snapshot.message}</p>}
        {error && <p className="error">{error}</p>}
        <p className="muted small">
          Starting here inserts into Voxelle itself, so the result is copied instead. To dictate into
          another app, focus it and use a shortcut
          {settingsQ.status === "ok" && (
            <>
              : toggle <kbd>{settingsQ.data.toggleHotkey}</kbd>
              {settingsQ.data.pushToTalkHotkey && (
                <>
                  , or hold <kbd>{settingsQ.data.pushToTalkHotkey}</kbd>
                </>
              )}
              {settingsQ.data.cancelHotkey && (
                <>
                  ; cancel with <kbd>{settingsQ.data.cancelHotkey}</kbd>
                </>
              )}
            </>
          )}
          .
        </p>
      </div>

      {lastResult && <LastResult result={lastResult} />}

      {!coreConnected && (
        <div className="notice">
          Running in browser preview — the desktop core is not connected, so dictation and live data
          are unavailable here.
        </div>
      )}
    </div>
  );
}
