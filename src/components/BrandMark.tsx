// Original waveform mark derived from the Voxelle logo (5 rounded bars on a rounded square).
// Derivation only — not a copy of any reference app asset. See docs/ui-direction.md.

export function BrandMark({ size = 28 }: { size?: number }) {
  const bars = [
    { x: 7, h: 10 },
    { x: 12.5, h: 18 },
    { x: 18, h: 26 },
    { x: 23.5, h: 18 },
    { x: 29, h: 10 },
  ];
  return (
    <svg
      className="brandmark"
      width={size}
      height={size}
      viewBox="0 0 40 40"
      role="img"
      aria-label="Voxelle"
    >
      <rect x="0" y="0" width="40" height="40" rx="10" fill="#101427" />
      {bars.map((b) => (
        <rect
          key={b.x}
          x={b.x}
          y={(40 - b.h) / 2}
          width="4"
          height={b.h}
          rx="2"
          fill="#f4f6ff"
        />
      ))}
    </svg>
  );
}
