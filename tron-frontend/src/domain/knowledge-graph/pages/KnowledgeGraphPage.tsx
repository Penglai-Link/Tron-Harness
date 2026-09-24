import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { Minimize2 } from "lucide-react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import {
  GraphCanvas,
  GraphInspector,
  GraphToolbar,
} from "@/domain/knowledge-graph/components";
import {
  useKnowledgeGraphExplorer,
  useKnowledgeGraphNamespaces,
} from "@/domain/knowledge-graph/hooks";
import type {
  GraphFacet,
  GraphLayoutMode,
  GraphNodeView,
  KnowledgeGraphExplorerFilters,
} from "@/domain/knowledge-graph/model";
import {
  defaultGraphNodeLabelProperty,
  graphNodePropertyOptions,
} from "@/domain/knowledge-graph/model";
import {
  PageHeader,
  SurfaceCard,
  useTabPanelConfig,
  useWorkspaceSpaces,
} from "@/shared";
import { Button } from "@/components/ui/button";

const defaultGraphLayoutMode: GraphLayoutMode = "Radial";
const preferredGraphNamespace = "rag_skill";

function pickGraphNamespace(namespaces: string[]) {
  if (namespaces.includes(preferredGraphNamespace)) return preferredGraphNamespace;
  return namespaces[0] ?? "";
}

function graphLabelFacets(nodes: GraphNodeView[]): GraphFacet[] {
  const counts = new Map<string, number>();
  for (const node of nodes) {
    for (const label of node.labels) {
      const normalizedLabel = label.trim();
      if (!normalizedLabel) continue;
      counts.set(normalizedLabel, (counts.get(normalizedLabel) ?? 0) + 1);
    }
  }

  return Array.from(counts.entries())
    .map(([label, count]) => ({
      count,
      label,
      value: label,
    }))
    .sort((left, right) => right.count - left.count || left.label.localeCompare(right.label));
}

export function GraphPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string) => t(`pages:knowledgeGraph.${key}`, { defaultValue });
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = selectedSecondarySpace?.workspaceCode
    ?? selectedPrimarySpace?.workspaceCode
    ?? "";
  const [selectedNodeCode, setSelectedNodeCode] = useState("");
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [graphFullscreenActive, setGraphFullscreenActive] = useState(false);
  const [graphLayoutMode, setGraphLayoutMode] = useState<GraphLayoutMode>(defaultGraphLayoutMode);
  const [nodeLabelProperty, setNodeLabelProperty] = useState(defaultGraphNodeLabelProperty);
  const [locatorQuery, setLocatorQuery] = useState("");
  const [filters, setFilters] = useState<KnowledgeGraphExplorerFilters>({
    labels: [],
    namespace: "",
    relationships: [],
    workspaceCode: activeWorkspaceCode,
  });
  const [zoom, setZoom] = useState(100);
  const [refreshTick, setRefreshTick] = useState(0);
  const namespaceState = useKnowledgeGraphNamespaces(activeWorkspaceCode);
  const namespaceOptions = namespaceState.namespaces;
  const graph = useKnowledgeGraphExplorer(filters, selectedNodeCode, refreshTick);
  const graphError = namespaceState.error ?? graph.error;
  const relationships = useMemo(
    () => Array.from(new Set(graph.filterOptionEdges.map((edge) => edge.relationship))).sort(),
    [graph.filterOptionEdges],
  );
  const nodeLabelProperties = useMemo(() => {
    const options = graphNodePropertyOptions(graph.filterOptionNodes);
    return options.includes(nodeLabelProperty)
      ? options
      : [nodeLabelProperty, ...options];
  }, [graph.filterOptionNodes, nodeLabelProperty]);
  const labelFacets = useMemo(
    () => graphLabelFacets(graph.filterOptionNodes),
    [graph.filterOptionNodes],
  );

  useTabPanelConfig({
    searchPlaceholder: pageT("shellSearch", "Locate graph node code or title..."),
  });

  useEffect(() => {
    setFilters((current) => {
      if (!activeWorkspaceCode) {
        return current.workspaceCode === "" && current.namespace === ""
          ? current
          : {
            ...current,
            namespace: "",
            workspaceCode: "",
          };
      }
      if (current.workspaceCode !== activeWorkspaceCode) {
        return {
          ...current,
          namespace: "",
          workspaceCode: activeWorkspaceCode,
        };
      }
      if (namespaceOptions.length === 0) {
        return current.namespace === ""
          ? current
          : {
            ...current,
            namespace: "",
          };
      }
      if (current.namespace && namespaceOptions.includes(current.namespace)) {
        return current;
      }

      return {
        ...current,
        namespace: pickGraphNamespace(namespaceOptions),
      };
    });
  }, [activeWorkspaceCode, namespaceOptions]);

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

  useEffect(() => {
    setNodeLabelProperty(defaultGraphNodeLabelProperty);
    setSelectedNodeCode("");
    setInspectorOpen(false);
    setLocatorQuery("");
  }, [filters.namespace]);

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
        highlightedEdgeCodes={graph.highlightedEdgeCodes}
        highlightedNodeCodes={graph.highlightedNodeCodes}
        layoutMode={graphLayoutMode}
        metrics={graph.metrics}
        nodeLabelProperty={nodeLabelProperty}
        nodes={graph.visibleNodes}
        selectedNodeCode={selectedNodeCode}
        zoom={zoom}
        onCameraZoomChange={syncCameraZoom}
        onClearSelection={clearSelectedNode}
        onSelectNode={selectNode}
      />
      {inspectorOpen && selectedNodeCode && graph.selectedNode ? (
        <div className="absolute bottom-3 right-3 top-3 z-30 w-[min(380px,calc(100%-1.5rem))] overflow-y-auto rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] shadow-[0_18px_48px_rgba(15,23,42,0.18)] backdrop-blur-md">
          <GraphInspector
            edges={graph.selectedNodeEdges}
            node={graph.selectedNode}
            scope={graph.scope}
            onClose={() => setInspectorOpen(false)}
            onExpandNode={(nodeCode) => {
              void graph.expandNode(nodeCode);
            }}
          />
        </div>
      ) : null}
    </div>
  );

  const graphPanel = (
    <SurfaceCard className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
      <GraphToolbar
        activeLabels={filters.labels}
        facets={labelFacets}
        fullscreenActive={graphFullscreenActive}
        layoutMode={graphLayoutMode}
        locatorNodes={graph.visibleNodes}
        locatorQuery={locatorQuery}
        nodeLabelProperties={nodeLabelProperties}
        nodeLabelProperty={nodeLabelProperty}
        namespace={filters.namespace}
        namespaces={namespaceOptions}
        relationships={relationships}
        selectedRelationships={filters.relationships}
        refreshing={graph.loading}
        onActiveLabelsChange={(labels) => setFilter("labels", labels)}
        onLocateNode={selectNode}
        onLocatorQueryChange={setLocatorQuery}
        onLayoutModeChange={changeGraphLayoutMode}
        onNamespaceChange={(namespace) => setFilter("namespace", namespace)}
        onNodeLabelPropertyChange={setNodeLabelProperty}
        onRefresh={() => setRefreshTick((current) => current + 1)}
        onSelectedRelationshipsChange={(relationships) => setFilter("relationships", relationships)}
        onToggleFullscreen={() => setGraphFullscreenActive((current) => !current)}
        onZoomChange={changeCameraZoom}
      />

      <div className="min-h-0 min-w-0 flex-1">
        {canvasPanel}
      </div>
    </SurfaceCard>
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        className="shrink-0"
        title={pageT("title", "Knowledge Graph")}
      />
      {graphError ? (
        <p
          className="mb-3 rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error)]/10 px-3 py-2 text-sm text-[var(--color-error)]"
          role="alert"
        >
          {graphError.message}
        </p>
      ) : null}

      {graphFullscreenActive && typeof document !== "undefined" ? createPortal(
        <div className="fixed inset-0 z-[1000] bg-[var(--color-surface-container-lowest)]">
          {canvasPanel}
          <Button
            className="absolute right-4 top-4 z-40 bg-[var(--color-elevated-surface)]/90 shadow-[0_10px_28px_rgba(15,23,42,0.10)] backdrop-blur-sm"
            size="icon"
            variant="ghost"
            type="button"
            aria-label={pageT("toolbar.fullscreen.exitAria", "Exit graph fullscreen")}
            title={pageT("toolbar.fullscreen.exit", "Exit fullscreen")}
            onClick={() => setGraphFullscreenActive(false)}
          >
            <Minimize2 size={16} aria-hidden="true" />
          </Button>
        </div>,
        document.body,
      ) : (
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          {graphPanel}
        </div>
      )}
    </div>
  );
}

export function KnowledgeGraphPage() {
  return <GraphPage />;
}
