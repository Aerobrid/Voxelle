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

  const history =
    q.status === "ok"
      ? q.data.storeTranscripts
        ? q.data.retentionDays == null
          ? "Kept until you delete it"
          : `Kept for ${q.data.retentionDays} days`
        : "Not stored"
      : "—";

  return (
    <div className="page">
      <div className="page__head">
        <h1>Privacy</h1>
        <p className="sub">What runs on this computer, what is kept, and what could ever use the network.</p>
      </div>

      <div className="tiles">
        <div className="tile">
          <div className="tile__label">Microphone</div>
          <div className="tile__value">
            <span className={`dot ${recording ? "dot--recording" : ""}`} />
            {recording ? "In use — recording now" : "Not in use"}
          </div>
        </div>
        <div className="tile">
          <div className="tile__label">Dictation</div>
          <div className="tile__value">
            <span className="dot dot--ready" />
            100% on this device
          </div>
        </div>
        <div className="tile">
          <div className="tile__label">Transcript history</div>
          <div className="tile__value">{history}</div>
          {info.status === "ok" && !info.data.persistent && (
            <div className="small error">Database unavailable — nothing is saved to disk this session.</div>
          )}
        </div>
        <div className="tile">
          <div className="tile__label">Audio recordings</div>
          <div className="tile__value">Never stored</div>
        </div>
      </div>

      <section className="section">
        <h2>Network</h2>
        <p className="muted" style={{ marginTop: 4 }}>
          Offline mode is on. Nothing in this version makes network requests: the dictation loop contains
          no network code. In-app model downloads (planned) will ask first and show what is contacted.
        </p>
      </section>

      <section className="section">
        <h2>Your data</h2>
        <div className="card" style={{ marginTop: 10 }}>
          {info.status === "ok" ? (
            <>
              <div className="tile__label">Data folder</div>
              <div className="path">{info.data.dataDir}</div>
              <div className="tile__label" style={{ marginTop: 10 }}>Models folder</div>
              <div className="path">{info.data.modelDir}</div>
            </>
          ) : (
            <p className="faint" style={{ margin: 0 }}>Live values are available inside the desktop app.</p>
          )}
          <div className="actions" style={{ marginTop: 14 }}>
            <button className="btn" onClick={exportHistory}>
              Export history
            </button>
            <button className="btn btn--ghost" onClick={() => setSection("history")}>
              Delete entries…
            </button>
            {note && <span className="muted small">{note}</span>}
          </div>
        </div>
      </section>

      <section className="section">
        <h2>Installed models</h2>
        {models.status === "ok" ? (
          models.data.filter((m) => m.installed).length === 0 ? (
            <p className="faint">None installed.</p>
          ) : (
            models.data
              .filter((m) => m.installed)
              .map((m) => (
                <div className="entry" key={m.id} style={{ gridTemplateColumns: "1fr auto" }}>
                  <div>
                    <div>{m.name}</div>
                    <div className="path">{m.path}</div>
                  </div>
                  <span className={m.integrityVerified ? "tag tag--ok" : "tag"}>
                    {m.integrityVerified ? "Verified · local" : "Not verified yet"}
                  </span>
                </div>
              ))
          )
        ) : (
          <p className="faint">Live values are available inside the desktop app.</p>
        )}
      </section>
    </div>
  );
}
