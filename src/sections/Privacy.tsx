import { useState } from "react";
import { core } from "../ipc/bridge";
import { errorText, useCore } from "../state/useCore";
import { useAppStore } from "../state/store";

export function Privacy() {
  const [q] = useCore(() => core.getSettings(), []);
  const [info] = useCore(() => core.storageInfo(), []);
  const [models] = useCore(() => core.listModels(), []);
  const setSection = useAppStore((s) => s.setSection);
  const snapshot = useAppStore((s) => s.snapshot);
  const recording = snapshot.state === "recording";
  const [note, setNote] = useState<string | null>(null);

  const exportHistory = () =>
    core.exportHistory().then(
      (path) => setNote(`History exported to ${path}`),
      (e) => setNote(errorText(e)),
    );

  return (
    <div className="main">
      <h1>Privacy</h1>
      <p className="sub">What runs locally, what could use the network, and what is retained.</p>

      <div className="card">
        <strong>Live status</strong>
        <p className="muted">
          Microphone in use: <span className="mono">{recording ? "yes — recording now" : "no"}</span>
        </p>
        <p className="muted">
          Dictation (hotkey → record → transcribe → insert) runs entirely on this device. It contains
          no network code.
        </p>
      </div>

      <div className="card">
        <strong>What is kept</strong>
        {q.status === "ok" && info.status === "ok" ? (
          <>
            <p className="muted">
              Transcript history:{" "}
              <span className="mono">
                {q.data.storeTranscripts
                  ? q.data.retentionDays == null
                    ? "kept until you delete it"
                    : `kept ${q.data.retentionDays} days`
                  : "not stored"}
              </span>
              {!info.data.persistent && " · database unavailable: nothing is saved to disk this session"}
            </p>
            <p className="muted">Audio recordings: <span className="mono">never stored</span></p>
            <p className="muted">
              Data folder: <span className="mono">{info.data.dataDir}</span>
            </p>
            <p className="muted">
              Models folder: <span className="mono">{info.data.modelDir}</span>
            </p>
            <div className="actions">
              <button className="btn" onClick={exportHistory}>
                Export history (JSON)
              </button>
              <button className="btn btn--ghost" onClick={() => setSection("history")}>
                Delete entries…
              </button>
              {note && <span className="muted">{note}</span>}
            </div>
          </>
        ) : (
          <p className="muted">Live values available inside the desktop app.</p>
        )}
      </div>

      <div className="card">
        <strong>Installed models</strong>
        {models.status === "ok" ? (
          models.data.filter((m) => m.installed).length === 0 ? (
            <p className="muted">None installed.</p>
          ) : (
            models.data
              .filter((m) => m.installed)
              .map((m) => (
                <p className="muted small" key={m.id}>
                  {m.name} · local · {m.integrityVerified ? "checksum verified" : "not verified yet"} ·{" "}
                  <span className="mono">{m.path}</span>
                </p>
              ))
          )
        ) : (
          <p className="muted">Live values available inside the desktop app.</p>
        )}
      </div>

      <div className="card">
        <strong>Network</strong>
        <p className="muted">
          Offline mode is on. No feature in this version makes network requests; in-app model
          downloads (planned) will ask first and show what is contacted.
        </p>
      </div>
    </div>
  );
}
