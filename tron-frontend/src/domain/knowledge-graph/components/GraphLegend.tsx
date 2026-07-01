const legendItems = [
  { color: "#1c1b1b", label: "RagSkill" },
  { color: "#2f5f9f", label: "RagTool" },
  { color: "#4f6f52", label: "RagTag" },
  { color: "#747878", label: "Text Unit" },
  { color: "#8a6a1f", label: "Vector" },
  { color: "#7a5e9a", label: "Ontology" },
] as const;

export function GraphLegend() {
  return (
    <div className="absolute bottom-4 right-4 z-20 flex flex-wrap items-center gap-3 rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2 shadow-[0_10px_28px_rgba(15,23,42,0.08)]">
      {legendItems.map((item) => (
        <span className="inline-flex items-center gap-1.5 text-[12px] text-[var(--color-on-surface-variant)]" key={item.label}>
          <span className="size-2.5 rounded-full" style={{ backgroundColor: item.color }} />
          {item.label}
        </span>
      ))}
    </div>
  );
}
