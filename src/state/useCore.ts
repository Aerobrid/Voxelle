import { useEffect, useState } from "react";
import { CoreNotConnected } from "../ipc/bridge";

export type CoreQuery<T> =
  | { status: "loading" }
  | { status: "ok"; data: T }
  | { status: "disconnected" }
  | { status: "error"; error: string };

/** Run a core call, mapping CoreNotConnected to an honest "disconnected" state (no fake data). */
export function useCore<T>(fn: () => Promise<T>, deps: unknown[] = []): CoreQuery<T> {
  const [state, setState] = useState<CoreQuery<T>>({ status: "loading" });
  useEffect(() => {
    let active = true;
    fn()
      .then((data) => active && setState({ status: "ok", data }))
      .catch((e: unknown) => {
        if (!active) return;
        if (e instanceof CoreNotConnected) setState({ status: "disconnected" });
        else setState({ status: "error", error: e instanceof Error ? e.message : String(e) });
      });
    return () => {
      active = false;
    };
    // deps are intentionally caller-controlled
  }, deps);
  return state;
}
