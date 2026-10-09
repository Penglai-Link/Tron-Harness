import { useEffect, useId, useRef, useState, type PointerEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  Plus,
  GitBranch,
  ZoomIn,
  ZoomOut,
  Maximize,
  Trash2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import type { LoopDesign } from "../model/LoopModel";
import { loopNodeName } from "../model/builtinLoops";
import type { CanvasPosition, LoopOutlet } from "../model/loopCanvas";
import { graphPositions } from "../model/loopValidation";

const NODE_WIDTH = 158,
  NODE_HEIGHT = 72;
type Wire = { index: number; outlet: LoopOutlet };
type Gesture =
  | {
      kind: "node";
      index: number;
      start: CanvasPosition;
      position: CanvasPosition;
    }
  | { kind: "pan"; start: CanvasPosition; position: CanvasPosition }
  | { kind: "wire"; wire: Wire };
const clampZoom = (zoom: number) => Math.max(0.001, Math.min(2, zoom));
function curve(a: CanvasPosition, b: CanvasPosition) {
  const bend = Math.max(70, Math.abs(b.x - a.x) / 2);
  return `M ${a.x} ${a.y} C ${a.x + bend} ${a.y}, ${b.x - bend} ${b.y}, ${
    b.x
  } ${b.y}`;
}

export function DagLoopCanvas({
  design,
  selected,
  disabled,
  onSelect,
  onMove,
  onConnect,
  onDisconnect,
  onAdd,
}: {
  design: LoopDesign;
  selected: number;
  disabled: boolean;
  onSelect: (index: number) => void;
  onMove: (index: number, position: CanvasPosition) => void;
  onConnect: (from: number, to: number, outlet: LoopOutlet) => void;
  onDisconnect: (edge: number) => void;
  onAdd: (kind: "Model" | "Branch", position: CanvasPosition) => void;
}) {
  const { t } = useTranslation("pages");
  const root = useRef<HTMLDivElement>(null);
  const gesture = useRef<Gesture | null>(null);
  const [camera, setCamera] = useState({ x: 24, y: 28, zoom: 1 });
  const [wire, setWire] = useState<Wire | null>(null);
  const [preview, setPreview] = useState<CanvasPosition | null>(null);
  const [edge, setEdge] = useState<number | null>(null);
  const arrow = useId().replace(/:/g, "");
  const positions = graphPositions(design);
  const point = (clientX: number, clientY: number): CanvasPosition => {
    const rect = root.current!.getBoundingClientRect();
    return {
      x: (clientX - rect.left - camera.x) / camera.zoom,
      y: (clientY - rect.top - camera.y) / camera.zoom,
    };
  };
  const output = (index: number, outlet: LoopOutlet): CanvasPosition => {
    const p = positions.get(design.nodes[index]?.id) ?? { x: 0, y: 0 };
    return {
      x: p.x + NODE_WIDTH,
      y: p.y + (outlet === "matched" ? 24 : outlet === "otherwise" ? 52 : 36),
    };
  };
  const center = () => ({
    x:
      ((root.current?.clientWidth ?? 500) / 2 - camera.x) / camera.zoom -
      NODE_WIDTH / 2 +
      (design.nodes.length % 5) * 16,
    y:
      ((root.current?.clientHeight ?? 520) / 2 - camera.y) / camera.zoom -
      NODE_HEIGHT / 2 +
      (design.nodes.length % 5) * 16,
  });
  function zoom(factor: number) {
    setCamera((current) => {
      const next = clampZoom(current.zoom * factor);
      const x = (root.current?.clientWidth ?? 500) / 2,
        y = (root.current?.clientHeight ?? 520) / 2;
      return {
        x: x - ((x - current.x) * next) / current.zoom,
        y: y - ((y - current.y) * next) / current.zoom,
        zoom: next,
      };
    });
  }
  function fit() {
    const values = [...positions.values()];
    if (!values.length) {
      setCamera({ x: 24, y: 28, zoom: 1 });
      return;
    }
    const minX = Math.min(...values.map((p) => p.x)),
      minY = Math.min(...values.map((p) => p.y));
    const width = Math.max(...values.map((p) => p.x)) - minX + NODE_WIDTH;
    const height = Math.max(...values.map((p) => p.y)) - minY + NODE_HEIGHT;
    const w = root.current?.clientWidth ?? 500,
      h = root.current?.clientHeight ?? 520;
    const next = Math.max(
      0.001,
      Math.min(1.5, (w - 64) / width, (h - 64) / height),
    );
    setCamera({
      x: (w - width * next) / 2 - minX * next,
      y: (h - height * next) / 2 - minY * next,
      zoom: next,
    });
  }
  const fitRef = useRef(fit);
  fitRef.current = fit;
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    let fitted = false;
    const observer = new ResizeObserver(() => {
      if (!fitted && element.clientWidth > 0) {
        fitRef.current();
        fitted = true;
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      const x = event.clientX - rect.left,
        y = event.clientY - rect.top;
      setCamera((current) => {
        const next = clampZoom(current.zoom * Math.exp(-event.deltaY * 0.002));
        return {
          x: x - ((x - current.x) * next) / current.zoom,
          y: y - ((y - current.y) * next) / current.zoom,
          zoom: next,
        };
      });
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  }, []);
  function capture(event: PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    root.current?.setPointerCapture(event.pointerId);
  }
  function startWire(event: PointerEvent, next: Wire) {
    if (disabled || event.button !== 0) return;
    capture(event);
    setEdge(null);
    setWire(next);
    setPreview(output(next.index, next.outlet));
    gesture.current = { kind: "wire", wire: next };
  }
  function finishWire(to: number) {
    if (wire && !disabled) onConnect(wire.index, to, wire.outlet);
    setWire(null);
    setPreview(null);
  }
  function move(event: PointerEvent) {
    const active = gesture.current;
    if (!active) return;
    if (active.kind === "wire") {
      setPreview(point(event.clientX, event.clientY));
      return;
    }
    const dx = event.clientX - active.start.x,
      dy = event.clientY - active.start.y;
    if (active.kind === "pan")
      setCamera((current) => ({
        ...current,
        x: active.position.x + dx,
        y: active.position.y + dy,
      }));
    else if (!disabled)
      onMove(active.index, {
        x: Math.max(
          -100000,
          Math.min(100000, Math.round(active.position.x + dx / camera.zoom)),
        ),
        y: Math.max(
          -100000,
          Math.min(100000, Math.round(active.position.y + dy / camera.zoom)),
        ),
      });
  }
  function end(event: PointerEvent) {
    if (gesture.current?.kind === "wire") {
      const target = document
        .elementFromPoint(event.clientX, event.clientY)
        ?.closest<HTMLElement>("[data-loop-input]");
      if (target && root.current?.contains(target))
        finishWire(Number(target.dataset.loopInput));
      else {
        setWire(null);
        setPreview(null);
      }
    }
    gesture.current = null;
    if (root.current?.hasPointerCapture(event.pointerId))
      root.current.releasePointerCapture(event.pointerId);
  }
  return (
    <div className="min-w-0 overflow-hidden rounded-xl border border-[var(--color-outline-variant)]">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-2">
        <div className="flex flex-wrap gap-1">
          {(["Model", "Branch"] as const).map((kind) => (
            <div
              key={kind}
              draggable={!disabled}
              onDragStart={(event) =>
                event.dataTransfer.setData("application/tron-loop-node", kind)
              }
            >
              <Button
                type="button"
                size="sm"
                variant="ghost"
                disabled={disabled || design.nodes.length >= 64}
                onClick={() => onAdd(kind, center())}
              >
                {kind === "Model" ? (
                  <Plus size={14} />
                ) : (
                  <GitBranch size={14} />
                )}
                {t(
                  kind === "Model"
                    ? "agentLoops.addModel"
                    : "agentLoops.addBranch",
                )}
              </Button>
            </div>
          ))}
        </div>
        <div className="flex items-center gap-1">
          {edge !== null && design.edges[edge] ? (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              disabled={disabled}
              aria-label={t("agentLoops.removeEdge")}
              onClick={() => {
                onDisconnect(edge);
                setEdge(null);
              }}
            >
              <Trash2 size={14} />
            </Button>
          ) : null}
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={t("agentLoops.zoomOut")}
            onClick={() => zoom(1 / 1.2)}
          >
            <ZoomOut size={14} />
          </Button>
          <span className="w-10 text-center text-xs text-[var(--color-on-surface-variant)]">
            {Math.round(camera.zoom * 1000) / 10}%
          </span>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={t("agentLoops.zoomIn")}
            onClick={() => zoom(1.2)}
          >
            <ZoomIn size={14} />
          </Button>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={t("agentLoops.fitCanvas")}
            onClick={fit}
          >
            <Maximize size={14} />
          </Button>
        </div>
      </div>
      <div
        ref={root}
        role="group"
        aria-label={t("agentLoops.graph")}
        tabIndex={0}
        className="relative h-[520px] touch-none overflow-hidden bg-[var(--color-surface-container-low)] outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--color-focus-ring)]"
        style={{
          backgroundImage:
            "radial-gradient(var(--color-outline-variant) 1px, transparent 1px)",
          backgroundSize: `${24 * camera.zoom}px ${24 * camera.zoom}px`,
          backgroundPosition: `${camera.x}px ${camera.y}px`,
        }}
        onDragOver={(event) => {
          if (
            !disabled &&
            event.dataTransfer.types.includes("application/tron-loop-node")
          ) {
            event.preventDefault();
            event.dataTransfer.dropEffect = "copy";
          }
        }}
        onDrop={(event) => {
          event.preventDefault();
          const kind = event.dataTransfer.getData("application/tron-loop-node");
          if (!disabled && (kind === "Model" || kind === "Branch"))
            onAdd(kind, point(event.clientX, event.clientY));
        }}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          capture(event);
          setEdge(null);
          setWire(null);
          setPreview(null);
          gesture.current = {
            kind: "pan",
            start: { x: event.clientX, y: event.clientY },
            position: { x: camera.x, y: camera.y },
          };
        }}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={() => {
          gesture.current = null;
          setWire(null);
          setPreview(null);
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            setWire(null);
            setPreview(null);
            setEdge(null);
          }
          if (
            (event.key === "Delete" || event.key === "Backspace") &&
            edge !== null &&
            !disabled
          ) {
            event.preventDefault();
            onDisconnect(edge);
            setEdge(null);
          }
        }}
      >
        <svg
          role="group"
          className="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
          aria-label={t("agentLoops.connections")}
        >
          <defs>
            <marker
              id={arrow}
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="7"
              markerHeight="7"
              orient="auto"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--color-outline)" />
            </marker>
          </defs>
          <g
            transform={`translate(${camera.x},${camera.y}) scale(${camera.zoom})`}
          >
            {design.edges.map((connection, index) => {
              const fromIndex = design.nodes.findIndex(
                  (node) => node.id === connection.from,
                ),
                target = positions.get(connection.to);
              if (fromIndex < 0 || !target) return null;
              const from = design.nodes[fromIndex];
              const outlet =
                from.kind === "Branch"
                  ? from.condition?.matchTarget === connection.to
                    ? "matched"
                    : "otherwise"
                  : "output";
              const a = output(fromIndex, outlet),
                b = { x: target.x, y: target.y + 36 };
              const label =
                outlet === "output"
                  ? ""
                  : t(
                      outlet === "matched"
                        ? "agentLoops.matched"
                        : "agentLoops.otherwise",
                    );
              return (
                <g
                  key={`${connection.from}-${connection.to}-${index}`}
                  role="button"
                  tabIndex={0}
                  aria-label={`${connection.from} → ${connection.to} ${label}`}
                  aria-pressed={edge === index}
                  className="pointer-events-auto cursor-pointer outline-none [&:focus-visible_path]:stroke-[var(--color-focus-ring)]"
                  onPointerDown={(event) => event.stopPropagation()}
                  onClick={() => {
                    setEdge(index);
                    setWire(null);
                    setPreview(null);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setEdge(index);
                    }
                  }}
                >
                  <path
                    d={curve(a, b)}
                    fill="none"
                    stroke="transparent"
                    strokeWidth="18"
                    style={{ pointerEvents: "stroke" }}
                  />
                  <path
                    d={curve(a, b)}
                    fill="none"
                    stroke={
                      edge === index
                        ? "var(--color-accent)"
                        : "var(--color-outline)"
                    }
                    strokeWidth={edge === index ? 2.5 : 1.5}
                    markerEnd={`url(#${arrow})`}
                    style={{ pointerEvents: "none" }}
                  />
                  {label ? (
                    <text
                      x={(a.x + b.x) / 2}
                      y={(a.y + b.y) / 2 - 7}
                      textAnchor="middle"
                      fontSize="11"
                      fill="var(--color-on-surface-variant)"
                    >
                      {label}
                    </text>
                  ) : null}
                </g>
              );
            })}
            {wire && preview ? (
              <path
                d={curve(output(wire.index, wire.outlet), preview)}
                fill="none"
                stroke="var(--color-accent)"
                strokeWidth="2"
                strokeDasharray="5 4"
              />
            ) : null}
          </g>
        </svg>
        <div
          className="absolute left-0 top-0"
          style={{
            transform: `translate(${camera.x}px, ${camera.y}px) scale(${camera.zoom})`,
            transformOrigin: "0 0",
          }}
        >
          {design.nodes.map((node, index) => {
            const position = positions.get(node.id)!;
            const outlets: LoopOutlet[] =
              node.kind === "Branch" ? ["matched", "otherwise"] : ["output"];
            return (
              <div
                key={index}
                role="button"
                tabIndex={0}
                aria-label={loopNodeName(design, node, t) || `${index + 1}`}
                aria-pressed={selected === index}
                className="absolute select-none rounded-lg border bg-[var(--color-surface-container-lowest)] px-3 py-3 shadow-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
                style={{
                  left: position.x,
                  top: position.y,
                  width: NODE_WIDTH,
                  height: NODE_HEIGHT,
                  cursor: disabled ? "pointer" : "grab",
                  borderColor:
                    selected === index
                      ? "var(--color-accent)"
                      : "var(--color-outline-variant)",
                  background:
                    selected === index
                      ? "var(--color-accent-container)"
                      : undefined,
                }}
                onPointerDown={(event) => {
                  if (event.button !== 0) return;
                  event.stopPropagation();
                  onSelect(index);
                  setEdge(null);
                  if (!disabled) {
                    capture(event);
                    gesture.current = {
                      kind: "node",
                      index,
                      start: { x: event.clientX, y: event.clientY },
                      position,
                    };
                  }
                }}
                onKeyDown={(event) => {
                  if (event.target !== event.currentTarget) return;
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onSelect(index);
                  }
                  if (
                    !disabled &&
                    [
                      "ArrowLeft",
                      "ArrowRight",
                      "ArrowUp",
                      "ArrowDown",
                    ].includes(event.key)
                  ) {
                    event.preventDefault();
                    onSelect(index);
                    const step = event.shiftKey ? 50 : 10;
                    onMove(index, {
                      x:
                        position.x +
                        (event.key === "ArrowLeft"
                          ? -step
                          : event.key === "ArrowRight"
                          ? step
                          : 0),
                      y:
                        position.y +
                        (event.key === "ArrowUp"
                          ? -step
                          : event.key === "ArrowDown"
                          ? step
                          : 0),
                    });
                  }
                }}
              >
                <span className="block truncate text-xs font-medium">
                  {loopNodeName(design, node, t) || "—"}
                </span>
                <span className="mt-1 block text-[10px] text-[var(--color-on-surface-variant)]">
                  {t(
                    node.kind === "Model"
                      ? "agentLoops.modelNode"
                      : "agentLoops.branchNode",
                  )}
                </span>
                <button
                  type="button"
                  disabled={disabled}
                  data-loop-input={index}
                  aria-label={t("agentLoops.inputPort", { node: node.id })}
                  title={t("agentLoops.inputPort", { node: node.id })}
                  className="absolute -left-2 top-7 size-4 rounded-full border-2 border-[var(--color-accent)] bg-[var(--color-surface-container-lowest)] outline-none hover:bg-[var(--color-accent-container)] focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
                  onPointerDown={(event) => event.stopPropagation()}
                  onClick={(event) => {
                    event.stopPropagation();
                    finishWire(index);
                  }}
                />
                {outlets.map((outlet) => (
                  <button
                    key={outlet}
                    type="button"
                    disabled={disabled}
                    aria-label={t("agentLoops.outputPort", {
                      node: node.id,
                      outlet: t(
                        outlet === "output"
                          ? "agentLoops.output"
                          : outlet === "matched"
                          ? "agentLoops.matched"
                          : "agentLoops.otherwise",
                      ),
                    })}
                    title={t(
                      outlet === "output"
                        ? "agentLoops.output"
                        : outlet === "matched"
                        ? "agentLoops.matched"
                        : "agentLoops.otherwise",
                    )}
                    data-loop-output={outlet}
                    className="absolute -right-2 size-4 rounded-full border-2 border-[var(--color-accent)] bg-[var(--color-surface-container-lowest)] outline-none hover:bg-[var(--color-accent-container)] focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
                    style={{
                      top:
                        (outlet === "matched"
                          ? 24
                          : outlet === "otherwise"
                          ? 52
                          : 36) - 8,
                    }}
                    onPointerDown={(event) =>
                      startWire(event, { index, outlet })
                    }
                    onClick={(event) => {
                      event.stopPropagation();
                      if (event.detail === 0) {
                        setWire({ index, outlet });
                        setPreview(output(index, outlet));
                      }
                    }}
                  >
                    {node.kind === "Branch" ? (
                      <span className="pointer-events-none absolute right-5 top-0 whitespace-nowrap text-[9px] leading-3">
                        {t(
                          outlet === "matched"
                            ? "agentLoops.matched"
                            : "agentLoops.otherwise",
                        )}
                      </span>
                    ) : null}
                  </button>
                ))}
              </div>
            );
          })}
        </div>
        <span className="pointer-events-none absolute bottom-3 left-3 rounded bg-[var(--color-surface)] px-2 py-1 text-[10px] text-[var(--color-on-surface-variant)]">
          {t(wire ? "agentLoops.connecting" : "agentLoops.canvasHelp")}
        </span>
      </div>
    </div>
  );
}
