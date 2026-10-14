import { core } from "../ipc/bridge";
import { useCore } from "../state/useCore";
import { Gate } from "../components/Gate";

export function History() {
  const q = useCore(() => core.getHistory(), []);
  return (
    <div className="main">
      <h1>History</h1>
      <p className="sub">Raw transcripts stay on your machine. Edited versions are kept separately.</p>
      <Gate q={q}>
        {(entries) =>
          entries.length === 0 ? (
            <div className="notice">No dictations yet.</div>
          ) : (
            <div>
              {entries.map((e) => (
                <div className="card" key={e.id}>
                  <div className="muted">
                    {new Date(e.createdAt).toLocaleString()} · {(e.durationMs / 1000).toFixed(1)}s
                    {e.language ? ` · ${e.language}` : ""}
                  </div>
                  <p className="mono">{e.rawTranscript}</p>
                </div>
              ))}
            </div>
          )
        }
      </Gate>
    </div>
  );
}
