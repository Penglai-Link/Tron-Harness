import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { SurfaceCard } from "@/shared";
import { listRagCorpora } from "@/domain/rag/api/ragApi";
import type { RagCorpus } from "@/domain/rag/model/RagModel";
import { GraphCanvas } from "./GraphCanvas";
import { GraphInspector } from "./GraphInspector";
import { useRagGraph } from "../hooks/useRagGraph";
import { defaultRagGraphOptions } from "../model/RagGraphModel";
import { ragCanvasNodes, ragCanvasEdges } from "../model/ragGraphWindow";
import type { RagGraphDirection, RagGraphOptions } from "../model/RagGraphModel";

// Start with a readable window; expansion retains the server's bounded traversal.
const initialGraphOptions = { ...defaultRagGraphOptions, seedCount: 3, nodeLimit: 25, edgeLimit: 40 };

export function RagGraphPanel() {
  const { t } = useTranslation("pages"); const [params] = useSearchParams();
  const initialCorpus = params.get("corpus") ?? ""; const initialQuery = params.get("query") ?? "";
  const [corpus, setCorpus] = useState(initialCorpus); const [query, setQuery] = useState(initialQuery);
  const [corpora, setCorpora] = useState<RagCorpus[]>([]); const [catalogError, setCatalogError] = useState<Error | null>(null);
  const [options, setOptions] = useState<RagGraphOptions>(initialGraphOptions);
  const [relations, setRelations] = useState(""); const [selected, setSelected] = useState(""); const [zoom, setZoom] = useState(100);
  const graph = useRagGraph(); const { request, reset } = graph;
  useEffect(() => {
    const controller = new AbortController(); listRagCorpora(controller.signal).then((rows) => { if (!controller.signal.aborted) setCorpora(rows.filter((row) => row.settings.graph)); })
      .catch((cause: unknown) => { if (!controller.signal.aborted) setCatalogError(cause instanceof Error ? cause : new Error(String(cause))); });
    return () => controller.abort();
  }, []);
  useEffect(() => {
    setCorpus(initialCorpus); setQuery(initialQuery); setSelected(""); reset();
    const body = { ...initialGraphOptions, corpusIds: initialCorpus ? [initialCorpus] : [], filters: {} };
    void request(initialQuery.trim() ? { ...body, query: initialQuery } : body, false);
  }, [initialCorpus, initialQuery, request, reset]);
  const nodes = useMemo(() => ragCanvasNodes(graph.window?.nodes ?? []), [graph.window?.nodes]);
  const edges = useMemo(() => ragCanvasEdges(graph.window?.edges ?? []), [graph.window?.edges]);
  const node = graph.window?.nodes.find((item) => item.id === selected);
  const connected = graph.window?.edges.filter((edge) => edge.from === selected || edge.to === selected) ?? [];
  const metrics = useMemo(() => ({ nodeCount: nodes.length, edgeCount: edges.length, namespaceCount: new Set(graph.window?.nodes.map((item) => item.corpusId)).size }), [nodes.length, edges.length, graph.window?.nodes]);
  const bounded = nodes.length >= 500 || edges.length >= 1000;
  const validDepth = [options.incomingDepth, options.outgoingDepth].every((depth) => Number.isInteger(depth) && depth >= 0 && depth <= 6);
  function configured(): RagGraphOptions { return { ...options, relationshipTypes: Array.from(new Set(relations.split(/[,，]/).map((value) => value.trim()).filter(Boolean))) }; }
  function search(nextCorpus = corpus) {
    setSelected("");
    const body = { ...configured(), corpusIds: nextCorpus ? [nextCorpus] : [], filters: {} };
    void request(query.trim() ? { ...body, query } : body, false);
  }
  function expand(direction: RagGraphDirection) {
    if (!node) return;
    void request({ ...configured(), incomingDepth: direction === "Incoming" ? options.incomingDepth : 0,
      outgoingDepth: direction === "Outgoing" ? options.outgoingDepth : 0, nodeIds: [node.id], corpusIds: corpus ? [corpus] : [], filters: {} }, true);
  }
  return <div className="flex min-h-0 flex-1 flex-col gap-3">
    <form className="grid gap-3 lg:grid-cols-[minmax(10rem,1fr)_minmax(12rem,2fr)_auto]" onSubmit={(event) => { event.preventDefault(); search(); }}>
      <Field><FieldLabel>{t("ragUnified.corpus")}</FieldLabel><SelectInput disabled={graph.loading} value={corpus || "__all__"} aria-label={t("ragUnified.corpus")} onValueChange={(value) => { const next = value === "__all__" ? "" : String(value); setCorpus(next); search(next); }}><SelectItem value="__all__">{t("knowledgeGraph.ragAllCorpora")}</SelectItem>{corpora.map((row) => <SelectItem value={row.id} key={row.id}>{row.name}</SelectItem>)}</SelectInput></Field>
      <Field><FieldLabel htmlFor="rag-graph-query">{t("ragUnified.question")}</FieldLabel><Input id="rag-graph-query" value={query} onChange={(event) => setQuery(event.target.value)} /></Field>
      <Button className="self-end" type="submit" disabled={graph.loading || !validDepth}>{t(graph.loading ? "ragUnified.searching" : query.trim() ? "ragUnified.search" : "knowledgeGraph.ragBrowse")}</Button>
    </form>
    <details><summary className="cursor-pointer text-sm">{t("knowledgeGraph.ragOptions")}</summary><div className="mt-3 grid gap-3 sm:grid-cols-3">
      {(["incomingDepth", "outgoingDepth"] as const).map((key) => <Field key={key}><FieldLabel htmlFor={`rag-graph-${key}`}>{t(`knowledgeGraph.ragDepth.${key}`)}</FieldLabel><Input id={`rag-graph-${key}`} type="number" min={0} max={6} value={options[key]} onChange={(event) => { setOptions((old) => ({ ...old, [key]: Number(event.target.value) })); reset(); setSelected(""); }} /></Field>)}
      <Field><FieldLabel htmlFor="rag-graph-relations">{t("knowledgeGraph.ragRelationFilter")}</FieldLabel><Input id="rag-graph-relations" value={relations} onChange={(event) => { setRelations(event.target.value); reset(); setSelected(""); }} /></Field>
    </div></details>
    {catalogError || graph.error ? <p role="alert" className="text-sm text-destructive">{(catalogError ?? graph.error)?.message}</p> : null}
    <div className="flex flex-wrap items-center justify-between gap-2 text-sm" aria-live="polite"><span>{t("knowledgeGraph.ragWindowCount", { nodes: nodes.length, edges: edges.length })}</span>
      {nodes.length ? <SelectInput className="w-full sm:w-72" aria-label={t("knowledgeGraph.ragInspectEntity")} placeholder={t("knowledgeGraph.ragInspectEntity")} value={selected || "__none__"} onValueChange={(value) => setSelected(value === "__none__" ? "" : String(value))}>
        <SelectItem value="__none__">{t("knowledgeGraph.ragInspectEntity")}</SelectItem>
        {graph.window?.nodes.map((item) => <SelectItem key={item.id} value={item.id}>{item.name}{item.entityType ? ` · ${item.entityType}` : ""}</SelectItem>)}
      </SelectInput> : null}
      {graph.window?.nextCursor ? <Button variant="outline" disabled={graph.loading || bounded} onClick={() => void request({ cursor: graph.window!.nextCursor! }, true)}>{t("knowledgeGraph.ragLoadMore")}</Button> : null}
    </div>
    {graph.window?.truncatedBy.length ? <p className="text-sm text-muted-foreground">{t("knowledgeGraph.ragBounded")}</p> : null}
    {bounded ? <p role="status" className="text-sm text-muted-foreground">{t("knowledgeGraph.ragClientBudget")}</p> : null}
    <SurfaceCard className="relative min-h-[32rem] flex-1 overflow-hidden !p-0">
      {!nodes.length && !graph.error ? <p role="status" className="absolute left-4 top-4 z-10 text-sm text-muted-foreground">{t(graph.loading ? "knowledgeGraph.ragLoading" : "knowledgeGraph.ragEmpty")}</p> : null}
      <GraphCanvas nodes={nodes} edges={edges} metrics={metrics} layoutMode="Radial" nodeLabelProperty="name" zoom={zoom} onCameraZoomChange={setZoom}
        selectedNodeCode={selected} onSelectNode={setSelected} onClearSelection={() => setSelected("")} highlightedNodeCodes={node ? [node.id] : []} highlightedEdgeCodes={connected.map((edge) => edge.id)} />
      {node ? <aside className="absolute inset-y-3 right-3 z-30 w-[min(380px,calc(100%-1.5rem))] overflow-y-auto rounded-xl border bg-[var(--color-elevated-surface)] shadow-lg"><GraphInspector ragNode={node} ragNodes={graph.window?.nodes ?? []} ragEdges={connected} ragReferences={graph.window?.references ?? []} pending={graph.loading || bounded || !validDepth}
        incomingDepth={options.incomingDepth} outgoingDepth={options.outgoingDepth} onClose={() => setSelected("")} onExpand={expand} onSelect={setSelected} /></aside> : null}
    </SurfaceCard>
  </div>;
}
