import { useEffect, useState } from "react";
import { core } from "../ipc/bridge";
import { canCancel, canStart, canStop } from "../ipc/transitions";
import type { DictationResult } from "../ipc/types";
import { toast, useAppStore } from "../state/store";
import { errorText, useCore } from "../state/useCore";
import { todayStats, wordsPerMinute } from "../state/derive";
import { VoiceOrb } from "../components/VoiceOrb";
import { KeyChips } from "../components/ShortcutRecorder";
import { IconCopy } from "../components/Icons";

const OUTCOME_TEXT: Record<string, string> = {
  inserted: "Inserted at the cursor",
  "copied-only": "Copied — paste with Ctrl+V",
  failed: "Not inserted",
};

function LastResult({ result, onStats }: { result: DictationResult; onStats: () => void }) {
  const [text, setText] = useState(result.transcript);
  useEffect(() => setText(result.transcript), [result]);

  const edited = text !== result.transcript;
  const copy = () =>
    core.copyText(text).then(
      () => toast("Copied to the clipboard."),
      (e) => toast(errorText(e), "error"),
    );
  const saveEdit = () => {
    if (!result.historyId) return;
    core.setHistoryEdit(result.historyId, text).then(
      () => toast("Correction saved. The original is kept too."),
      (e) => toast(errorText(e), "error"),
    );
  };

  return (
    <section className="section">
      <div className="section__head">
        <h2>Last dictation</h2>
        <span className={`outcome outcome--${result.insert.outcome}`}>
          <span className="dot" style={{ background: "currentColor" }} />
          {OUTCOME_TEXT[result.insert.outcome]}
        </span>
      </div>
      <div className="card">
        {result.insert.message && <p className="muted small" style={{ margin: 0 }}>{result.insert.message}</p>}
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
            <IconCopy />
            {edited ? "Copy corrected text" : "Copy"}
          </button>
          <button className="btn btn--ghost" onClick={saveEdit} disabled={!edited || !result.historyId}>
            Save correction
          </button>
          {edited && (
            <button className="btn btn--ghost" onClick={() => setText(result.transcript)}>
              Revert
            </button>
          )}
          <span className="actions__spacer" />
          <button className="link small" onClick={onStats}>
            Timings
          </button>
        </div>
      </div>
    </section>
  );
}

function heroTitle(state: string, micReady: boolean): string {
  switch (state) {
    case "recording":
      return micReady ? "Listening…" : "Starting your microphone…";
    case "processing":
      return "Transcribing…";
    case "inserting":
      return "Inserting…";
    default:
      return "Ready when you are";
  }
}

export function Dashboard() {
  const snapshot = useAppStore((s) => s.snapshot);
  const coreConnected = useAppStore((s) => s.coreConnected);
  const lastResult = useAppStore((s) => s.lastResult);
  const setSection = useAppStore((s) => s.setSection);
  const [settingsQ] = useCore(() => core.getSettings(), [snapshot.state === "idle"]);
  const [modelsQ] = useCore(() => core.listModels(), []);
  const [historyQ] = useCore(() => core.getHistory(), [lastResult]);
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
  const stats = historyQ.status === "ok" ? todayStats(historyQ.data) : null;
  const settings = settingsQ.status === "ok" ? settingsQ.data : null;
  const pace = stats ? wordsPerMinute(stats.words, stats.audioMs) : null;

  const orbClick = () => {
    if (canStop(s)) run(core.stopDictation());
    else if (canStart(s)) run(core.startDictation());
  };

  return (
    <div className="page">
      {coreConnected && modelsQ.status === "ok" && !selected && (
        <div className="notice notice--warn">
          {anyInstalled ? "No speech model selected. " : "No speech model installed yet. "}
          <button className="link" onClick={() => setSection("models")}>
            Open Models
          </button>
        </div>
      )}
      {!coreConnected && (
        <div className="notice">
          Browser preview — the desktop core isn't connected, so dictation and live data are unavailable.
        </div>
      )}

      <div className="hero">
        <VoiceOrb
          snapshot={snapshot}
          disabled={!coreConnected || !(canStart(s) || canStop(s))}
          onClick={orbClick}
          label={canStop(s) ? "Stop and transcribe" : "Start dictation in this window"}
        />
        <div className="hero__title">{heroTitle(s, snapshot.micReady)}</div>
        {s === "recording" ? (
          <p className="hero__hint">
            <span className="mono">{(snapshot.elapsedMs / 1000).toFixed(1)} s</span>
            <button className="btn btn--ghost btn--sm" disabled={!canCancel(s)} onClick={() => run(core.cancelDictation())}>
              Cancel
            </button>
          </p>
        ) : settings ? (
          <p className="hero__hint">
            {settings.pushToTalkHotkey && (
              <>
                Hold <KeyChips shortcut={settings.pushToTalkHotkey} /> in any app
                <span className="faint">·</span>
              </>
            )}
            press <KeyChips shortcut={settings.toggleHotkey} /> for hands-free
          </p>
        ) : null}
        {snapshot.message && <p className="hero__msg muted">{snapshot.message}</p>}
        {error && <p className="hero__msg error">{error}</p>}
        {!error && !snapshot.message && s !== "recording" && (
          <p className="hero__msg faint">
            Clicking the orb dictates into Voxelle itself, so the result is copied. To type into another
            app, use a shortcut there.
          </p>
        )}
      </div>

      <div className="stats">
        <div className="stat">
          <div className="stat__value">{stats ? stats.words : "—"}</div>
          <div className="stat__label">Words today</div>
        </div>
        <div className="stat">
          <div className="stat__value">{stats ? stats.dictations : "—"}</div>
          <div className="stat__label">Dictations today</div>
        </div>
        <div className="stat">
          <div className="stat__value">{pace ?? "—"}</div>
          <div className="stat__label">Words per minute today</div>
        </div>
      </div>

      {lastResult && <LastResult result={lastResult} onStats={() => setSection("stats")} />}

      {historyQ.status === "ok" && historyQ.data.length > 0 && !lastResult && (
        <section className="section">
          <div className="section__head">
            <h2>Recent</h2>
            <button className="link small" onClick={() => setSection("history")}>
              View all
            </button>
          </div>
          {historyQ.data.slice(0, 3).map((e) => (
            <div className="entry" key={e.id} style={{ gridTemplateColumns: "64px 1fr" }}>
              <span className="entry__time">
                {new Date(e.createdAt).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}
              </span>
              <p className="entry__text">{e.editedText ?? e.rawTranscript}</p>
            </div>
          ))}
        </section>
      )}
    </div>
  );
}
