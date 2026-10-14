import { useCallback, useEffect, useState } from "react";
import { CoreNotConnected } from "../ipc/bridge";

export type CoreQuery<T> =
  | { status: "loading" }
  | { status: "ok"; data: T }
  | { status: "disconnected" }
  | { status: "error"; error: string };

export function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/**
 * Run a core call, mapping CoreNotConnected to an honest "disconnected" state (no fake data).
 * Returns the query state and a `reload` function for after mutations.
 */
export function useCore<T>(fn: () => Promise<T>, deps: unknown[] = []): [CoreQuery<T>, () => void] {
  const [state, setState] = useState<CoreQuery<T>>({ status: "loading" });
  const [version, setVersion] = useState(0);
  const reload = useCallback(() => setVersion((v) => v + 1), []);
  useEffect(() => {
    let active = true;
    fn()
      .then((data) => active && setState({ status: "ok", data }))
      .catch((e: unknown) => {
        if (!active) return;
        if (e instanceof CoreNotConnected) setState({ status: "disconnected" });
        else setState({ status: "error", error: errorText(e) });
      });
    return () => {
      active = false;
    };
    // deps are intentionally caller-controlled
  }, [...deps, version]);
  return [state, reload];
}
