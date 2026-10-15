export function Planned({ title, note }: { title: string; note: string }) {
  return (
    <div className="page">
      <div className="page__head">
        <h1>
          {title} <span className="tag tag--soon" style={{ verticalAlign: "middle" }}>planned</span>
        </h1>
        <p className="sub">{note}</p>
      </div>
      <div className="empty">Not built yet. This page shows no made-up data.</div>
    </div>
  );
}
