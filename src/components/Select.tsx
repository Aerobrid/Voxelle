// Dropdown drawn by us: a native <select> opens an OS-styled square list that can't match the
// app's rounded surfaces. Keyboard: Up/Down/Home/End move, Enter/Space choose, Escape/Tab close,
// typing a letter jumps to the next option starting with it.
import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";

export interface SelectOption {
  value: string;
  label: string;
}

/** Index of the next option whose label starts with `ch`, after `from` (wrapping). */
export function typeAhead(options: SelectOption[], from: number, ch: string): number {
  const c = ch.toLowerCase();
  for (let step = 1; step <= options.length; step++) {
    const i = (from + step) % options.length;
    if (options[i].label.toLowerCase().startsWith(c)) return i;
  }
  return from;
}

export function Select({
  value,
  options,
  onChange,
  label,
  disabled,
}: {
  value: string;
  options: SelectOption[];
  onChange: (v: string) => void;
  /** Accessible name (the visible label lives in the settings row). */
  label: string;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [upward, setUpward] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const id = useId();
  const selectedIndex = Math.max(0, options.findIndex((o) => o.value === value));
  const current = options[selectedIndex];

  const show = () => {
    setActive(selectedIndex);
    setOpen(true);
  };
  const close = (refocus = true) => {
    setOpen(false);
    if (refocus) button.current?.focus();
  };
  const choose = (i: number) => {
    close();
    if (options[i] && options[i].value !== value) onChange(options[i].value);
  };

  // Open upward when the list would run off the bottom of the window.
  useLayoutEffect(() => {
    if (!open || !root.current || !list.current) return;
    const r = root.current.getBoundingClientRect();
    setUpward(r.bottom + list.current.offsetHeight + 12 > window.innerHeight && r.top > list.current.offsetHeight);
    list.current.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) close(false);
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open]);

  useEffect(() => {
    list.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active, open]);

  const onButtonKey = (e: KeyboardEvent) => {
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
      e.preventDefault();
      show();
    }
  };

  const onListKey = (e: KeyboardEvent) => {
    const last = options.length - 1;
    const moves: Record<string, number> = {
      ArrowDown: Math.min(last, active + 1),
      ArrowUp: Math.max(0, active - 1),
      Home: 0,
      End: last,
    };
    if (e.key in moves) {
      e.preventDefault();
      setActive(moves[e.key]);
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      choose(active);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === "Tab") {
      close(false);
    } else if (e.key.length === 1) {
      setActive(typeAhead(options, active, e.key));
    }
  };

  return (
    <div className={`select${open ? " select--open" : ""}`} ref={root}>
      <button
        type="button"
        ref={button}
        className="select__button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`${label}: ${current?.label ?? ""}`}
        disabled={disabled}
        onClick={() => (open ? close() : show())}
        onKeyDown={onButtonKey}
      >
        <span className="select__value">{current?.label}</span>
        <svg className="select__chevron" viewBox="0 0 10 6" aria-hidden>
          <path d="M1 1l4 4 4-4" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      {open && (
        <ul
          ref={list}
          className={`select__list${upward ? " select__list--up" : ""}`}
          role="listbox"
          tabIndex={-1}
          aria-label={label}
          aria-activedescendant={`${id}-${active}`}
          onKeyDown={onListKey}
        >
          {options.map((o, i) => (
            <li
              key={o.value}
              id={`${id}-${i}`}
              data-index={i}
              role="option"
              aria-selected={o.value === value}
              className={`select__option${i === active ? " select__option--active" : ""}`}
              onMouseEnter={() => setActive(i)}
              onClick={() => choose(i)}
            >
              <span>{o.label}</span>
              {o.value === value && (
                <svg viewBox="0 0 24 24" className="select__tick" aria-hidden>
                  <path d="M5 12.5l4.5 4.5L19 7.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
