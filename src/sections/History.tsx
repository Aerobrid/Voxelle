import { useState } from "react";
import { core } from "../ipc/bridge";
import { errorText, useCore } from "../state/useCore";
import { Gate } from "../components/Gate";
import { toast, useAppStore } from "../state/store";
import { groupByDay, matchesQuery } from "../state/derive";
import { IconCopy, IconTrash } from "../components/Icons";

export function History() {
  const lastResult = useAppStore((s) => s.lastResult);
  const [q, reload] = useCore(() => core.getHistory(), [lastResult]);
  const [query, setQuery] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);

  const act = (p: Promise<unknown>, ok: string, refresh = true) =>
    p.then(
      () => {
        toast(ok);
        if (refresh) reload();
      },
      (e) => toast(errorText(e), "error"),
    );

  return (
    <div className="page">
      <div className="page__head row-between" style={{ alignItems: "flex-end" }}>
        <div>
          <h1>History</h1>
          <p className="sub">Kept only on this device. Your corrections are saved next to the original transcript.</p>
        </div>
        {q.status === "ok" && q.data.length > 0 && (
          <div className="actions">
            <button
              className="btn btn--ghost btn--sm"
              onClick={() => core.exportHistory().then((p) => toast(`Exported to ${p}`), (e) => toast(errorText(e), "error"))}
            >
              Export
            </button>
            {confirmClear ? (
              <>
                <button
                  className="btn btn--danger btn--sm"
                  onClick={() => {
                    setConfirmClear(false);
                    act(core.clearHistory(), "History cleared.");
                  }}
                >
                  Delete all {q.data.length}
                </button>
                <button className="btn btn--ghost btn--sm" onClick={() => setConfirmClear(false)}>
                  Keep
                </button>
              </>
            ) : (
              <button className="btn btn--ghost btn--sm" onClick={() => setConfirmClear(true)}>
                Clear…
              </button>
            )}
          </div>
        )}
      </div>

      <Gate q={q}>
        {(entries) => {
          if (entries.length === 0) {
            return <div className="empty">Nothing here yet. Your dictations will appear here.</div>;
          }
          const shown = entries.filter((e) => matchesQuery(e, query));
          return (
            <>
              <input
                className="input search"
                placeholder="Search your dictations"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                aria-label="Search history"
              />
              {shown.length === 0 && <p className="faint" style={{ marginTop: 18 }}>No dictation matches “{query}”.</p>}
              {groupByDay(shown).map((g) => (
                <div key={g.label}>
                  <div className="day">{g.label}</div>
                  {g.entries.map((e) => (
                    <div className="entry" key={e.id}>
                      <span className="entry__time">
                        {new Date(e.createdAt).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}
                      </span>
                      <div>
                        <p className="entry__text">{e.editedText ?? e.rawTranscript}</p>
                        {e.editedText && <p className="entry__orig">Original: {e.rawTranscript}</p>}
                      </div>
                      <span className="entry__actions">
                        <button
                          className="icon-btn"
                          title="Copy"
                          aria-label="Copy"
                          onClick={() => act(core.copyText(e.editedText ?? e.rawTranscript), "Copied to the clipboard.", false)}
                        >
                          <IconCopy />
                        </button>
                        <button
                          className="icon-btn"
                          title="Delete"
                          aria-label="Delete"
                          onClick={() => act(core.deleteHistoryEntry(e.id), "Deleted.")}
                        >
                          <IconTrash />
                        </button>
                      </span>
                    </div>
                  ))}
                </div>
              ))}
            </>
          );
        }}
      </Gate>
    </div>
  );
}
