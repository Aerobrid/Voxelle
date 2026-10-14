import { core } from "../ipc/bridge";
import { useCore } from "../state/useCore";
import { useAppStore } from "../state/store";

export function Privacy() {
  const q = useCore(() => core.getSettings(), []);
  const snapshot = useAppStore((s) => s.snapshot);
  const recording = snapshot.state === "recording";

  return (
    <div className="main">
      <h1>Privacy</h1>
      <p className="sub">What runs locally, what could use the network, and what is retained.</p>

      <div className="card">
        <strong>Dictation loop</strong>
        <p className="muted">
          Hotkey → record → local transcription → insert. Fully local. The core loop never uses the
          network.
        </p>
      </div>

      <div className="card">
        <strong>Live status</strong>
        <p className="muted">
          Recording active: <span className="mono">{recording ? "yes" : "no"}</span>
        </p>
      </div>

      <div className="card">
        <strong>Retention &amp; network</strong>
        {q.status === "ok" ? (
          <>
            <p className="muted">
              Transcript storage: <span className="mono">{String(q.data.storeTranscripts)}</span> ·
              Retention:{" "}
              <span className="mono">
                {q.data.retentionDays == null ? "forever" : `${q.data.retentionDays}d`}
              </span>
            </p>
            <p className="muted">
              Offline mode: <span className="mono">{String(q.data.offlineMode)}</span> · Model dir:{" "}
              <span className="mono">{q.data.modelDir ?? "default"}</span>
            </p>
          </>
        ) : (
          <p className="muted">Live values available inside the desktop app.</p>
        )}
      </div>

      <div className="notice">
        Export and deletion controls, and the full installed-model inventory, land with the storage
        slice of P0.
      </div>
    </div>
  );
}
