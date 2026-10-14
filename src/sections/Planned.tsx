export function Planned({ title, note }: { title: string; note: string }) {
  return (
    <div className="main">
      <h1>{title}</h1>
      <p className="sub">
        <span className="rail__tag">planned</span>
      </p>
      <div className="notice">{note} This section is not implemented yet and shows no fabricated data.</div>
    </div>
  );
}
