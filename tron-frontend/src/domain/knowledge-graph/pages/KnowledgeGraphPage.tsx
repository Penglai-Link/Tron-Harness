import {
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  Download,
  GitBranch,
  Minimize2,
  RefreshCw,
} from "lucide-react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import {
  GraphCanvas,
  GraphInspector,
  GraphResultRail,
  GraphToolbar,
} from "@/domain/knowledge-graph/components";
import { useKnowledgeGraphExplorer } from "@/domain/knowledge-graph/hooks";
import type {
  GraphLayoutMode,
  KnowledgeGraphExplorerFilters,
} from "@/domain/knowledge-graph/model";
import {
  Button,
  PageHeader,
  SegmentedTabs,
  SurfaceCard,
  useConsoleShellConfig,
} from "@/shared";

const defaultGraphLayoutMode: GraphLayoutMode = "Radial";

export function GraphPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string) => t(`pages:knowledgeGraph.${key}`, { defaultValue });
  const [selectedNodeCode, setSelectedNodeCode] = useState("RAG_REVIEWER");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [graphFullscreenActive, setGraphFullscreenActive] = useState(false);
  const [graphLayoutMode, setGraphLayoutMode] = useState<GraphLayoutMode>(defaultGraphLayoutMode);
  const [filters, setFilters] = useState<KnowledgeGraphExplorerFilters>({
    labels: [],
    namespace: "rag_skill",
    query: "",
    relationship: "all",
    workspaceCode: "TRON_CORE",
  });
  const [zoom, setZoom] = useState(100);
  const [tab, setTab] = useState("explore");
  const graph = useKnowledgeGraphExplorer(filters, selectedNodeCode);
  const relationships = useMemo(
    () => Array.from(new Set(graph.edges.map((edge) => edge.relationship))).sort(),
    [graph.edges],
  );

  useConsoleShellConfig({
    searchPlaceholder: pageT("shellSearch", "Search graph nodes and relationships..."),
  });

  useEffect(() => {
    if (!graphFullscreenActive) return undefined;

    const previousOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setGraphFullscreenActive(false);
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [graphFullscreenActive]);

  function selectNode(nodeCode: string) {
    setSelectedNodeCode(nodeCode);
    setInspectorOpen(true);
  }

  function clearSelectedNode() {
    setSelectedNodeCode("");
    setInspectorOpen(false);
  }

  function syncCameraZoom(nextZoom: number) {
    setZoom((current) => (current === nextZoom ? current : nextZoom));
  }

  function changeCameraZoom(delta: number) {
    setZoom((current) => Math.min(220, Math.max(60, current + delta)));
  }

  function changeGraphLayoutMode(layoutMode: GraphLayoutMode) {
    setGraphLayoutMode(layoutMode);
  }

  function setFilter<K extends keyof KnowledgeGraphExplorerFilters>(key: K, value: KnowledgeGraphExplorerFilters[K]) {
    setFilters((current) => ({
      ...current,
      [key]: value,
    }));
  }

  const canvasPanel = (
    <div className="relative h-full min-h-0 min-w-0">
      <GraphCanvas
        edges={graph.visibleEdges}
        highlightedEdgeCodes={selectedNodeCode ? graph.highlightedEdgeCodes : []}
        highlightedNodeCodes={selectedNodeCode ? graph.highlightedNodeCodes : []}
        layoutMode={graphLayoutMode}
        nodes={graph.visibleNodes}
        selectedNodeCode={selectedNodeCode}
        zoom={zoom}
        onCameraZoomChange={syncCameraZoom}
        onClearSelection={clearSelectedNode}
        onSelectNode={selectNode}
      />
      {inspectorOpen && selectedNodeCode ? (
        <div className="absolute bottom-3 right-3 top-3 z-30 w-[min(380px,calc(100%-1.5rem))] overflow-y-auto rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] shadow-[0_18px_48px_rgba(15,23,42,0.18)] backdrop-blur-md">
          <GraphInspector
            edges={graph.selectedNodeEdges}
            node={graph.selectedNode}
            scope={graph.scope}
            onClose={() => setInspectorOpen(false)}
          />
        </div>
      ) : null}
    </div>
  );

  const graphPanel = (
    <SurfaceCard className="overflow-hidden p-0">
      <GraphToolbar
        activeLabels={filters.labels}
        facets={graph.facets}
        fullscreenActive={graphFullscreenActive}
        layoutMode={graphLayoutMode}
        metrics={graph.metrics}
        namespace={filters.namespace}
        namespaces={graph.namespaces}
        query={filters.query}
        relationship={filters.relationship}
        relationships={relationships}
        workspaceCode={filters.workspaceCode}
        onActiveLabelsChange={(labels) => setFilter("labels", labels)}
        onLayoutModeChange={changeGraphLayoutMode}
        onNamespaceChange={(namespace) => setFilter("namespace", namespace)}
        onQueryChange={(query) => setFilter("query", query)}
        onToggleFullscreen={() => setGraphFullscreenActive((current) => !current)}
        onZoomChange={changeCameraZoom}
        setRelationship={(relationship) => setFilter("relationship", relationship)}
      />

      <div className="grid min-h-[620px] xl:grid-cols-[280px_minmax(0,1fr)]">
        <GraphResultRail
          nodes={graph.resultNodes}
          selectedNodeCode={selectedNodeCode}
          onSelectNode={selectNode}
        />
        {canvasPanel}
      </div>
    </SurfaceCard>
  );

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] flex-col gap-6">
      <PageHeader
        title={pageT("title", "Knowledge Graph")}
        actions={(
          <>
            <SegmentedTabs
              value={tab}
              onValueChange={setTab}
              items={[
                { label: pageT("tabs.explore", "Explore"), value: "explore" },
                { label: pageT("tabs.search", "Search"), value: "search" },
                { label: pageT("tabs.ontology", "Ontology"), value: "ontology" },
              ]}
            />
            <Button type="button" variant="secondary">
              <GitBranch size={16} aria-hidden="true" />
              {pageT("actions.runTraversal", "Run Traversal")}
            </Button>
            <Button type="button" variant="secondary">
              <Download size={16} aria-hidden="true" />
              {pageT("actions.export", "Export")}
            </Button>
            <Button type="button">
              <RefreshCw size={16} aria-hidden="true" />
              {pageT("actions.refresh", "Refresh Graph")}
            </Button>
          </>
        )}
      />

      {graphFullscreenActive && typeof document !== "undefined" ? createPortal(
        <div className="fixed inset-0 z-[1000] bg-[var(--color-surface-container-lowest)]">
          {canvasPanel}
          <Button
            className="absolute right-4 top-4 z-40 bg-white/90 shadow-[0_10px_28px_rgba(15,23,42,0.10)] backdrop-blur-sm"
            size="icon"
            variant="ghost"
            type="button"
            aria-label="Exit graph fullscreen"
            title="Exit fullscreen"
            onClick={() => setGraphFullscreenActive(false)}
          >
            <Minimize2 size={16} aria-hidden="true" />
          </Button>
        </div>,
        document.body,
      ) : (
        <div className="min-w-0">
          {graphPanel}
        </div>
      )}
    </div>
  );
}

export function KnowledgeGraphPage() {
  return <GraphPage />;
}
