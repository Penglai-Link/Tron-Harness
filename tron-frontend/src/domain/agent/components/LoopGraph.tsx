import { useTranslation } from "react-i18next";
import { useId } from "react";
import type { LoopDesign } from "../model/LoopModel";
import { loopNodeName } from "../model/builtinLoops";
import { graphPositions, visibleEdges } from "../model/loopValidation";
export function LoopGraph({
  design,
  selected,
  onSelect,
}: {
  design: LoopDesign;
  selected: number;
  onSelect: (index: number) => void;
}) {
  const { t } = useTranslation("pages");
  const arrow = useId().replace(/:/g, "");
  const positions = graphPositions(design);
  const width = Math.max(480, ...[...positions.values()].map((p) => p.x + 180));
  const height = Math.max(170, ...[...positions.values()].map((p) => p.y + 78));
  return (
    <div
      className="overflow-auto rounded-xl bg-[var(--color-surface-container-low)] p-3"
      aria-label={t("agentLoops.graph")}
    >
      {design.kind === "React" ? (
        <p className="py-10 text-center text-sm text-[var(--color-on-surface-variant)]">
          {t("agentLoops.reactHelp")}
        </p>
      ) : (
        <svg
          role="group"
          aria-label={t("agentLoops.graph")}
          width={width}
          height={height}
        >
          <defs>
            <marker
              id={arrow}
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="6"
              markerHeight="6"
              orient="auto"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--color-outline)" />
            </marker>
          </defs>
          {visibleEdges(design).map((e, i) => {
            const a = positions.get(e.from),
              b = positions.get(e.to);
            if (!a || !b) return null;
            const from = design.nodes.find((n) => n.id === e.from);
            const label = from?.condition
              ? t(
                  from.condition.matchTarget === e.to
                    ? "agentLoops.matched"
                    : "agentLoops.otherwise",
                )
              : "";
            const middle = (a.x + 158 + b.x) / 2;
            return (
              <g key={`${e.from}-${e.to}-${i}`}>
                <path
                  d={`M ${a.x + 158} ${a.y + 26} C ${middle} ${
                    a.y + 26
                  }, ${middle} ${b.y + 26}, ${b.x} ${b.y + 26}`}
                  fill="none"
                  stroke="var(--color-outline)"
                  strokeWidth="1.5"
                  markerEnd={`url(#${arrow})`}
                />
                <title>
                  {e.from} → {e.to} {label}
                </title>
                {label ? (
                  <text
                    x={middle}
                    y={(a.y + b.y) / 2 + 15}
                    fontSize="10"
                    fill="var(--color-on-surface-variant)"
                  >
                    {label}
                  </text>
                ) : null}
              </g>
            );
          })}
          {design.nodes.map((n, i) => {
            const p = positions.get(n.id)!;
            const label = loopNodeName(design, n, t);
            return (
              <g
                key={i}
                role="button"
                tabIndex={0}
                aria-label={label || `${i + 1}`}
                aria-pressed={selected === i}
                onClick={() => onSelect(i)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onSelect(i);
                  }
                }}
                className="cursor-pointer outline-none [&:focus_rect]:stroke-[var(--color-focus-ring)]"
              >
                <rect
                  x={p.x}
                  y={p.y}
                  width="158"
                  height="52"
                  rx={n.kind === "Branch" ? 22 : 8}
                  fill={
                    selected === i
                      ? "var(--color-accent-container)"
                      : "var(--color-surface-container-lowest)"
                  }
                  stroke={
                    selected === i
                      ? "var(--color-accent)"
                      : "var(--color-outline-variant)"
                  }
                />
                <text
                  x={p.x + 12}
                  y={p.y + 20}
                  fontSize="12"
                  fill="var(--color-on-surface)"
                >
                  {label.length > 19 ? `${label.slice(0, 18)}…` : label || "—"}
                </text>
                <text
                  x={p.x + 12}
                  y={p.y + 38}
                  fontSize="10"
                  fill="var(--color-on-surface-variant)"
                >
                  {t(
                    `agentLoops.${
                      n.kind === "Model" ? "modelNode" : "branchNode"
                    }`,
                  )}
                </text>
              </g>
            );
          })}
        </svg>
      )}
    </div>
  );
}
