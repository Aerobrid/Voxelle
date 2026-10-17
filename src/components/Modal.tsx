// Small centered dialog. Closes on backdrop click; Escape is left to the caller because the
// shortcut recorder receives Escape itself (the keyboard hook takes every key while recording).
import { useEffect, useRef, type ReactNode } from "react";

export function Modal({
  title,
  onClose,
  children,
  closeOnEscape = true,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  closeOnEscape?: boolean;
}) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    box.current?.focus();
    if (!closeOnEscape) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, closeOnEscape]);

  return (
    <div className="modal__backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal" role="dialog" aria-modal="true" aria-label={title} tabIndex={-1} ref={box}>
        <h2 className="modal__title">{title}</h2>
        {children}
      </div>
    </div>
  );
}
