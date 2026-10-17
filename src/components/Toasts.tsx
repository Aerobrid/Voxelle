// Short confirmations ("Copied", "Shortcut saved") shown bottom-center, announced to screen readers.
import { useAppStore } from "../state/store";
import { IconAlert, IconCheck } from "./Icons";

export function Toasts() {
  const toasts = useAppStore((s) => s.toasts);
  const dismiss = useAppStore((s) => s.dismissToast);
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <button key={t.id} className={`toast toast--${t.tone}`} onClick={() => dismiss(t.id)}>
          {t.tone === "error" ? <IconAlert /> : <IconCheck />}
          <span>{t.text}</span>
        </button>
      ))}
    </div>
  );
}
