// Frameless window chrome: drag region, brand, live state, and window buttons. Close hides the
// window to the tray (core setting), so shortcuts keep working.
import { useEffect, useState } from "react";
import { isTauri } from "../ipc/bridge";
import { useAppStore } from "../state/store";
import { BrandMark } from "./BrandMark";
import { GlyphClose, GlyphMaximize, GlyphMinimize, GlyphRestore } from "./Icons";
import { stateLabel } from "../state/derive";

async function win() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export function TitleBar() {
  const snapshot = useAppStore((s) => s.snapshot);
  const coreConnected = useAppStore((s) => s.coreConnected);
  const [maximized, setMaximized] = useState(false);
  const { text, dot } = stateLabel(snapshot, coreConnected);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten = () => {};
    win().then(async (w) => {
      setMaximized(await w.isMaximized());
      unlisten = await w.onResized(async () => setMaximized(await w.isMaximized()));
    });
    return () => unlisten();
  }, []);

  return (
    <header className="titlebar">
      <div className="titlebar__brand">
        <BrandMark size={20} />
        Voxelle
      </div>
      <div className="titlebar__drag" data-tauri-drag-region onDoubleClick={() => win().then((w) => w.toggleMaximize())} />
      <span className="titlebar__state" aria-live="polite">
        <span className={`dot ${dot}`} />
        {text}
      </span>
      {isTauri() && (
        <div className="winbtns">
          <button className="winbtn" aria-label="Minimize" onClick={() => win().then((w) => w.minimize())}>
            <GlyphMinimize />
          </button>
          <button
            className="winbtn"
            aria-label={maximized ? "Restore" : "Maximize"}
            onClick={() => win().then((w) => w.toggleMaximize())}
          >
            {maximized ? <GlyphRestore /> : <GlyphMaximize />}
          </button>
          <button className="winbtn winbtn--close" aria-label="Close" onClick={() => win().then((w) => w.close())}>
            <GlyphClose />
          </button>
        </div>
      )}
    </header>
  );
}
