import { useState } from "react";
import { core } from "../ipc/bridge";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { useAppStore } from "../state/store";

export function History() {
  const lastResult = useAppStore((s) => s.lastResult);
  const [q, reload] = useCore(() => core.getHistory(), [lastResult]);
  const [note, setNote] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  const act = (p: Promise<unknown>, ok: string | null) =>
    p.then(
      () => {
        setNote(ok);
        reload();
      },
      (e) => setNote(errorText(e)),
    );

  return (
    <div className="main">
      <h1>History</h1>
      <p className="sub">
        Stored only on this device. The original transcript is kept; your corrections are saved
        alongside it.
      </p>
      {note && <p className="muted">{note}</p>}
      <Gate q={q}>
        {(entries) =>
          entries.length === 0 ? (
            <div className="notice">No dictations yet.</div>
          ) : (
            <div>
              <div className="actions">
                {confirmClear ? (
                  <>
                    <span>Delete all {entries.length} entries?</span>
                    <button
                      className="btn btn--danger"
                      onClick={() => {
                        setConfirmClear(false);
                        act(core.clearHistory(), "History cleared.");
                      }}
                    >
                      Delete all
                    </button>
                    <button className="btn btn--ghost" onClick={() => setConfirmClear(false)}>
                      Keep
                    </button>
                  </>
                ) : (
                  <button className="btn btn--ghost" onClick={() => setConfirmClear(true)}>
                    Clear history…
                  </button>
                )}
                <button
                  className="btn btn--ghost"
                  onClick={() => core.exportHistory().then((p) => setNote(`Exported to ${p}`), (e) => setNote(errorText(e)))}
                >
                  Export (JSON)
                </button>
              </div>
              {entries.map((e) => (
                <div className="card" key={e.id}>
                  <div className="row-between">
                    <span className="muted small">
                      {new Date(e.createdAt).toLocaleString()} · {(e.durationMs / 1000).toFixed(1)} s
                      {e.language ? ` · ${e.language}` : ""}
                    </span>
                    <span className="actions">
                      <button
                        className="btn btn--small"
                        onClick={() => act(core.copyText(e.editedText ?? e.rawTranscript), "Copied.")}
                      >
                        Copy
                      </button>
                      <button
                        className="btn btn--small btn--ghost"
                        onClick={() => act(core.deleteHistoryEntry(e.id), "Deleted.")}
                      >
                        Delete
                      </button>
                    </span>
                  </div>
                  {e.editedText ? (
                    <>
                      <p className="transcript">{e.editedText}</p>
                      <p className="muted small">
                        Original: <span className="transcript">{e.rawTranscript}</span>
                      </p>
                    </>
                  ) : (
                    <p className="transcript">{e.rawTranscript}</p>
                  )}
                </div>
              ))}
            </div>
          )
        }
      </Gate>
    </div>
  );
}
