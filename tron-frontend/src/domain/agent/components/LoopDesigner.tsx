import { ArrowUp, ArrowDown } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Field, SelectInput, TextInput, TextArea } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { useLoopDesignList } from "../hooks/useLoopDesignList";
import { useLoopDesignActions } from "../hooks/useLoopDesignActions";
import {
  newLoopDesign,
  type LoopDesign,
  type LoopKind,
  type LoopNode,
} from "../model/LoopModel";
import { validateLoop, visibleEdges } from "../model/loopValidation";
import { LoopGraph } from "./LoopGraph";
import { DagLoopCanvas } from "./DagLoopCanvas";
import {
  connectLoopNodes,
  disconnectLoopEdge,
  clearNodeReferences,
  withCanvasPositions,
  LoopConnectionError,
  type CanvasPosition,
} from "../model/loopCanvas";

import {
  isBuiltinLoop,
  loopName,
  loopDescription,
  copyLoopDesign,
} from "../model/builtinLoops";
import { BranchEditor } from "./BranchEditor";

type LoopSource = "all" | "builtin" | "custom";

function matchesSource(design: LoopDesign, source: LoopSource) {
  return (
    source === "all" ||
    isBuiltinLoop(design.loopCode) === (source === "builtin")
  );
}

export function LoopDesigner() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { designs, loading, error, refresh } = useLoopDesignList();
  const actions = useLoopDesignActions();
  const [draft, setDraft] = useState<LoopDesign | null>(null);
  const [original, setOriginal] = useState("");
  const [selected, setSelected] = useState(0);
  const [busy, setBusy] = useState(false);
  const [jsonInvalid, setJsonInvalid] = useState(false);
  const [source, setSource] = useState<LoopSource>("all");
  const [pending, setPending] = useState<{
    design: LoopDesign | null;
    source: LoopSource;
  } | null>(null);
  const [deleting, setDeleting] = useState(false);
  const filteredDesigns = useMemo(
    () => designs.filter((design) => matchesSource(design, source)),
    [designs, source],
  );
  const dirty = Boolean(draft && JSON.stringify(draft) !== original);
  const builtin = Boolean(draft && isBuiltinLoop(draft.loopCode));
  const issue = draft && !builtin ? validateLoop(draft) : null;
  const node = draft?.nodes[selected];
  const open = useCallback((design: LoopDesign) => {
    setDraft(structuredClone(design));
    setOriginal(JSON.stringify(design));
    setSelected(0);
    setJsonInvalid(false);
  }, []);
  useEffect(() => {
    if (!draft && !loading && !error && filteredDesigns[0]) {
      open(filteredDesigns[0]);
    }
  }, [draft, loading, error, filteredDesigns, open]);
  function navigate(design: LoopDesign | null, nextSource: LoopSource) {
    if (dirty) setPending({ design, source: nextSource });
    else {
      setSource(nextSource);
      if (design) open(design);
      else setDraft(null);
    }
  }
  function select(design: LoopDesign) {
    navigate(
      design,
      design.version === 0 && source === "builtin" ? "custom" : source,
    );
  }
  function filter(nextSource: LoopSource) {
    if (draft && matchesSource(draft, nextSource)) setSource(nextSource);
    else
      navigate(
        designs.find((design) => matchesSource(design, nextSource)) ?? null,
        nextSource,
      );
  }
  function patch(value: Partial<LoopDesign>) {
    setDraft((d) => (d ? { ...d, ...value } : d));
  }
  function updateNode(value: Partial<LoopNode>) {
    setDraft((d) => {
      if (!d) return d;
      const old = d.nodes[selected],
        next = { ...old, ...value };
      const edges = d.edges.map((e) => ({
        from: e.from === old.id ? next.id : e.from,
        to: e.to === old.id ? next.id : e.to,
      }));
      const nodes = d.nodes.map((n, i) =>
        i === selected
          ? next
          : n.condition
          ? {
              ...n,
              condition: {
                ...n.condition,
                sourceNode:
                  n.condition.sourceNode === old.id
                    ? next.id
                    : n.condition.sourceNode,
                matchTarget:
                  n.condition.matchTarget === old.id
                    ? next.id
                    : n.condition.matchTarget,
                elseTarget:
                  n.condition.elseTarget === old.id
                    ? next.id
                    : n.condition.elseTarget,
              },
            }
          : n,
      );
      return { ...d, nodes, edges };
    });
  }
  function updateBranch(c: NonNullable<LoopNode["condition"]>) {
    if (!draft || !node) return;
    const edges = draft.edges.filter(
      (e) =>
        e.from !== node.id &&
        !(e.to === node.id && e.from === node.condition?.sourceNode),
    );
    if (c.sourceNode) edges.push({ from: c.sourceNode, to: node.id });
    if (c.matchTarget) edges.push({ from: node.id, to: c.matchTarget });
    if (c.elseTarget) edges.push({ from: node.id, to: c.elseTarget });
    patch({
      nodes: draft.nodes.map((n, i) =>
        i === selected ? { ...n, condition: c } : n,
      ),
      edges,
    });
  }
  function add(kind: "Model" | "Branch", position?: CanvasPosition) {
    if (!draft || draft.nodes.length >= 64) return;
    let number = draft.nodes.length + 1;
    while (draft.nodes.some((n) => n.id === `node-${number}`)) number++;
    const next: LoopNode = {
      id: `node-${number}`,
      kind,
      instructions: "",
      join: "Selected",
      position: position ?? null,
      condition:
        kind === "Branch"
          ? {
              sourceNode: "",
              pointer: "",
              equals: true,
              matchTarget: "",
              elseTarget: "",
            }
          : null,
    };
    const existing =
      draft.kind === "Dag" ? withCanvasPositions(draft).nodes : draft.nodes;
    patch({ nodes: [...existing, next] });
    setSelected(draft.nodes.length);
    setJsonInvalid(false);
  }
  function removeNode() {
    if (!draft || !node) return;
    patch({
      nodes: clearNodeReferences(
        draft.nodes.filter((_, i) => i !== selected),
        node.id,
      ),
      edges: draft.edges.filter((e) => e.from !== node.id && e.to !== node.id),
    });
    setSelected(Math.max(0, selected - 1));
    setJsonInvalid(false);
  }
  async function save() {
    if (!draft || issue || jsonInvalid || builtin || busy) return;
    setBusy(true);
    try {
      open(await actions.save(draft));
      toast.success(t("agentLoops.saved"));
    } catch (e) {
      toast.error(e instanceof Error ? e.message : t("agentLoops.saveFailed"));
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    if (!draft || busy || builtin) return;
    setBusy(true);
    try {
      await actions.remove(draft.loopCode);
      setDraft(null);
      setDeleting(false);
      toast.success(t("agentLoops.deleted"));
    } catch (e) {
      toast.error(
        e instanceof Error ? e.message : t("agentLoops.deleteFailed"),
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="space-y-4" aria-label={t("agentLoops.title")}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div
          role="group"
          aria-label={t("agentLoops.sourceFilter")}
          className="flex gap-1"
        >
          {(["all", "builtin", "custom"] as const).map((value) => (
            <Button
              key={value}
              variant={source === value ? "secondary" : "ghost"}
              aria-pressed={source === value}
              disabled={busy}
              onClick={() => filter(value)}
            >
              {t(`agentLoops.sources.${value}`)}
            </Button>
          ))}
        </div>
        <Button
          disabled={busy || loading || Boolean(error)}
          onClick={() => select(newLoopDesign())}
        >

          {t("agentLoops.new")}
        </Button>
      </div>
      {error ? (
        <div
          role="alert"
          className="flex items-center gap-3 text-sm text-[var(--color-error)]"
        >
          {error}
          <Button variant="outline" onClick={() => void refresh()}>
            {t("agentLoops.refresh")}
          </Button>
        </div>
      ) : null}
      {loading ? (
        <p role="status" className="text-sm">
          {t("agentLoops.loading")}
        </p>
      ) : null}
      <div className="grid gap-5 lg:grid-cols-[220px_minmax(0,1fr)]">
        <nav aria-label={t("agentLoops.catalog")} className="space-y-1">
          {!loading && !error && filteredDesigns.length === 0 ? (
            <p className="px-3 py-3 text-sm text-[var(--color-on-surface-variant)]">
              {t("agentLoops.emptyCatalog")}
            </p>
          ) : null}
          {filteredDesigns.map((d) => (
            <button
              key={d.loopCode}
              type="button"
              disabled={busy}
              onClick={() => select(d)}
              aria-current={draft?.loopCode === d.loopCode ? "true" : undefined}
              className="w-full rounded-lg px-3 py-3 text-left transition-colors hover:bg-[var(--color-hover-surface)] aria-[current=true]:bg-[var(--color-accent-container)]"
            >
              <span className="flex items-center justify-between gap-2">
                <span className="min-w-0 break-words text-sm font-medium">
                  {loopName(d, t)}
                </span>
                <Badge variant="outline">
                  {t(
                    `agentLoops.sources.${
                      isBuiltinLoop(d.loopCode) ? "builtin" : "custom"
                    }`,
                  )}
                </Badge>
              </span>
              <span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">
                {t(`agentLoops.kinds.${d.kind}`)} · v{d.version}
              </span>
            </button>
          ))}
        </nav>
        {draft ? (
          <form
            className="min-w-0 space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            {builtin ? (
              <div className="flex flex-wrap items-center justify-between gap-3">
                <p className="text-sm text-[var(--color-on-surface-variant)]">
                  {loopDescription(draft, t)}
                </p>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    select(
                      copyLoopDesign(
                        draft,
                        t("agentLoops.copyName", { name: loopName(draft, t) }),
                      ),
                    )
                  }
                >
                  {t("agentLoops.copy")}
                </Button>
              </div>
            ) : null}
            <fieldset
              disabled={busy}
              className="min-w-0 space-y-4 disabled:opacity-80"
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label={t("agentLoops.code")} required>
                  <TextInput
                    aria-label={t("agentLoops.code")}
                    value={draft.loopCode}
                    maxLength={64}
                    disabled={draft.version > 0}
                    onChange={(e) => patch({ loopCode: e.target.value })}
                  />
                </Field>
                <Field label={t("agentLoops.name")} required>
                  <TextInput
                    aria-label={t("agentLoops.name")}
                    value={loopName(draft, t)}
                    disabled={builtin}
                    maxLength={100}
                    onChange={(e) => patch({ name: e.target.value })}
                  />
                </Field>
                <Field label={t("agentLoops.kind")}>
                  <SelectInput
                    aria-label={t("agentLoops.kind")}
                    value={draft.kind}
                    disabled={draft.version > 0}
                    onValueChange={(v) => {
                      const kind = v as LoopKind;
                      patch({
                        kind,
                        nodes:
                          kind === "React"
                            ? []
                            : (draft.nodes.length
                                ? draft.nodes
                                : newLoopDesign().nodes
                              ).map((n) => ({
                                ...n,
                                kind: "Model",
                                condition: null,
                                join: "Selected",
                              })),
                        edges: kind === "Dag" ? visibleEdges(draft) : [],
                        maxParallelNodes: 1,
                      });
                      setSelected(0);
                      setJsonInvalid(false);
                    }}
                  >
                    {(["React", "Workflow", "Dag"] as const).map((k) => (
                      <SelectItem key={k} value={k}>
                        {t(`agentLoops.kinds.${k}`)}
                      </SelectItem>
                    ))}
                  </SelectInput>
                </Field>
                {draft.kind === "Dag" ? (
                  <Field label={t("agentLoops.parallel")}>
                    <TextInput
                      aria-label={t("agentLoops.parallel")}
                      type="number"
                      min={1}
                      max={16}
                      step={1}
                      value={draft.maxParallelNodes}
                      disabled={builtin}
                      onChange={(e) =>
                        patch({
                          maxParallelNodes: e.currentTarget.valueAsNumber,
                        })
                      }
                    />
                  </Field>
                ) : null}
              </div>
              <div
                className={
                  draft.kind === "Dag"
                    ? "grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1fr)_260px]"
                    : "space-y-4"
                }
              >
                <div className="min-w-0 space-y-4">
                  {draft.kind === "Dag" ? (
                    <DagLoopCanvas
                      key={original}
                      design={draft}
                      selected={selected}
                      disabled={busy || Boolean(builtin)}
                      onSelect={(index) => {
                        setSelected(index);
                        setJsonInvalid(false);
                      }}
                      onMove={(index, position) => {
                        const next = withCanvasPositions(draft);
                        patch({
                          nodes: next.nodes.map((node, i) =>
                            i === index ? { ...node, position } : node,
                          ),
                        });
                      }}
                      onConnect={(from, to, outlet) => {
                        try {
                          patch(connectLoopNodes(draft, from, to, outlet));
                        } catch (error) {
                          toast.error(
                            t(
                              `agentLoops.connectionErrors.${
                                error instanceof LoopConnectionError
                                  ? error.reason
                                  : "edges"
                              }`,
                            ),
                          );
                        }
                      }}
                      onDisconnect={(edge) =>
                        patch(disconnectLoopEdge(draft, edge))
                      }
                      onAdd={add}
                    />
                  ) : (
                    <LoopGraph
                      design={draft}
                      selected={selected}
                      onSelect={(index) => {
                        setSelected(index);
                        setJsonInvalid(false);
                      }}
                    />
                  )}
                  {draft.kind === "Workflow" ? (
                    <Button
                      type="button"
                      variant="outline"
                      disabled={builtin || draft.nodes.length >= 64}
                      onClick={() => add("Model")}
                    >

                      {t("agentLoops.addModel")}
                    </Button>
                  ) : null}
                </div>
                {draft.kind !== "React" && node ? (
                  <fieldset
                    disabled={builtin}
                    className={
                      draft.kind === "Dag"
                        ? "max-h-[570px] space-y-4 overflow-y-auto border-l border-[var(--color-outline-variant)] pl-4"
                        : "space-y-4 border-t border-[var(--color-outline-variant)] pt-4"
                    }
                  >
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <h3 className="text-sm font-semibold">
                        {t("agentLoops.nodeSettings")}
                      </h3>
                      <div className="flex gap-2">
                        {draft.kind === "Workflow"
                          ? [-1, 1].map((offset) => (
                              <Button
                                key={offset}
                                type="button"
                                size="sm"
                                variant="ghost"
                                aria-label={t(
                                  offset < 0
                                    ? "agentLoops.moveUp"
                                    : "agentLoops.moveDown",
                                )}
                                disabled={
                                  selected + offset < 0 ||
                                  selected + offset >= draft.nodes.length
                                }
                                onClick={() => {
                                  const nodes = [...draft.nodes];
                                  [nodes[selected], nodes[selected + offset]] =
                                    [nodes[selected + offset], nodes[selected]];
                                  patch({ nodes });
                                  setSelected(selected + offset);
                                }}
                              >
                                {offset < 0 ? (
                                  <ArrowUp size={16} />
                                ) : (
                                  <ArrowDown size={16} />
                                )}
                              </Button>
                            ))
                          : null}
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={removeNode}
                        >

                          {t("agentLoops.removeNode")}
                        </Button>
                      </div>
                    </div>
                    <Field label={t("agentLoops.nodeId")}>
                      <TextInput
                        aria-label={t("agentLoops.nodeId")}
                        value={node.id}
                        maxLength={64}
                        onChange={(e) => updateNode({ id: e.target.value })}
                      />
                    </Field>
                    {node.kind === "Model" ? (
                      <Field label={t("agentLoops.instructions")}>
                        <TextArea
                          aria-label={t("agentLoops.instructions")}
                          rows={6}
                          value={node.instructions}
                          onChange={(e) =>
                            updateNode({ instructions: e.target.value })
                          }
                        />
                      </Field>
                    ) : (
                      <BranchEditor
                        key={`${selected}:${node.id}`}
                        condition={node.condition!}
                        design={draft}
                        onChange={updateBranch}
                        onInvalid={setJsonInvalid}
                      />
                    )}
                    {draft.kind === "Dag" ? (
                      <>
                        <Field label={t("agentLoops.join")}>
                          <SelectInput
                            aria-label={t("agentLoops.join")}
                            value={node.join}
                            onValueChange={(v) =>
                              updateNode({ join: v as LoopNode["join"] })
                            }
                          >
                            <SelectItem value="Selected">
                              {t("agentLoops.joinSelected")}
                            </SelectItem>
                            <SelectItem value="All">
                              {t("agentLoops.joinAll")}
                            </SelectItem>
                          </SelectInput>
                        </Field>
                      </>
                    ) : null}
                  </fieldset>
                ) : null}
              </div>
            </fieldset>
            {!builtin ? (
              <>
                {issue || jsonInvalid ? (
                  <p
                    role="status"
                    className="text-sm text-[var(--color-error)]"
                  >
                    {t(`agentLoops.validation.${jsonInvalid ? "json" : issue}`)}
                  </p>
                ) : null}
                <div className="flex items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-4">
                  <div>
                    {draft.version > 0 ? (
                      <Button
                        type="button"
                        variant="ghost"
                        disabled={busy}
                        onClick={() => setDeleting(true)}
                      >
                        {t("agentLoops.delete")}
                      </Button>
                    ) : null}
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-[var(--color-on-surface-variant)]">
                      {t(
                        dirty
                          ? "agentLoops.unsaved"
                          : "agentLoops.currentVersion",
                        { version: draft.version },
                      )}
                    </span>
                    <Button
                      type="submit"
                      disabled={busy || Boolean(issue) || jsonInvalid || !dirty}
                    >
                      {t(busy ? "agentLoops.saving" : "agentLoops.save")}
                    </Button>
                  </div>
                </div>
              </>
            ) : null}
          </form>
        ) : null}
      </div>
      <Dialog
        open={Boolean(pending)}
        onOpenChange={(v) => {
          if (!v) setPending(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("agentLoops.discardTitle")}</DialogTitle>
            <DialogDescription>{t("agentLoops.discardHelp")}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPending(null)}>
              {t("agentLoops.cancel")}
            </Button>
            <Button
              onClick={() => {
                if (pending) {
                  setSource(pending.source);
                  if (pending.design) open(pending.design);
                  else setDraft(null);
                }
                setPending(null);
              }}
            >
              {t("agentLoops.discard")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog
        open={deleting}
        onOpenChange={(v) => {
          if (!busy) setDeleting(v);
        }}
      >
        <DialogContent showCloseButton={!busy}>
          <DialogHeader>
            <DialogTitle>
              {t("agentLoops.deleteTitle", { name: draft?.name })}
            </DialogTitle>
            <DialogDescription>{t("agentLoops.deleteHelp")}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              disabled={busy}
              onClick={() => setDeleting(false)}
            >
              {t("agentLoops.cancel")}
            </Button>
            <Button disabled={busy} onClick={() => void remove()}>
              {t("agentLoops.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
