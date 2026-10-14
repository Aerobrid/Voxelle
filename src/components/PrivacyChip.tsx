import type { DataLocality } from "../ipc/types";

// Local/remote indicator shown on every operation that could use the network (principles #4).
export function PrivacyChip({ locality }: { locality: DataLocality }) {
  return (
    <span className={`chip chip--${locality}`}>
      <span className="dot" />
      {locality === "local" ? "Local" : "Can use network"}
    </span>
  );
}
