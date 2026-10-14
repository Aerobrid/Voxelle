import type { ReactNode } from "react";
import type { CoreQuery } from "../state/useCore";

/** Renders standard loading/disconnected/error states; delegates "ok" to the caller. */
export function Gate<T>({ q, children }: { q: CoreQuery<T>; children: (data: T) => ReactNode }) {
  switch (q.status) {
    case "loading":
      return <div className="muted">Loading…</div>;
    case "disconnected":
      return (
        <div className="notice">
          Core not connected. This view is live only inside the Voxelle desktop app.
        </div>
      );
    case "error":
      return <div className="notice">Error: {q.error}</div>;
    case "ok":
      return <>{children(q.data)}</>;
  }
}
