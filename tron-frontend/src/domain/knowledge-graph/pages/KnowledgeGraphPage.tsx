import {
  useMemo,
  useState,
} from "react";
import {
  Download,
  GitBranch,
  RefreshCw,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  GraphCanvas,
  GraphInspector,
  GraphResultRail,
  GraphToolbar,
} from "@/domain/knowledge-graph/components";
import { useKnowledgeGraphExplorer } from "@/domain/knowledge-graph/hooks";
import type { KnowledgeGraphExplorerFilters } from "@/domain/knowledge-graph/model";
import {
  Button,
  MetricCard,
  PageHeader,
  SegmentedTabs,
  SurfaceCard,
  useConsoleShellConfig,
  useOpenRightSidebar,
} from "@/shared";

function numberText(value: number) {
  return new Intl.NumberFormat("en-US").format(value);
}

export function GraphPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string) => t(`pages:knowledgeGraph.${key}`, { defaultValue });
  const openRightSidebar = useOpenRightSidebar();
  const [selectedNodeCode, setSelectedNodeCode] = useState("RAG_REVIEWER");
  const [inspectorOpen, setInspectorOpen] = useState(true);
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
  const rightPanel = useMemo(
    () => (inspectorOpen ? (
      <GraphInspector
        edges={graph.selectedNodeEdges}
        node={graph.selectedNode}
        scope={graph.scope}
        onClose={() => setInspectorOpen(false)}
      />
    ) : undefined),
    [graph.scope, graph.selectedNode, graph.selectedNodeEdges, inspectorOpen],
  );

  useConsoleShellConfig({
    rightPanel,
    searchPlaceholder: pageT("shellSearch", "Search graph nodes and relationships..."),
  });

  function selectNode(nodeCode: string) {
    setSelectedNodeCode(nodeCode);
    setInspectorOpen(true);
    openRightSidebar();
  }

  function setFilter<K extends keyof KnowledgeGraphExplorerFilters>(key: K, value: KnowledgeGraphExplorerFilters[K]) {
    setFilters((current) => ({
      ...current,
      [key]: value,
    }));
  }

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

      <section className="grid gap-4 xl:grid-cols-3" aria-label={pageT("metrics.aria", "Knowledge graph metrics")}>
        <MetricCard
          label={pageT("metrics.nodes", "Nodes")}
          value={numberText(graph.metrics.nodeCount)}
          trend={pageT("metrics.nodesTrend", "visible workspace lineage")}
        />
        <MetricCard
          label={pageT("metrics.edges", "Edges")}
          value={numberText(graph.metrics.edgeCount)}
          trend={pageT("metrics.edgesTrend", "indexed relationships")}
        />
        <MetricCard
          label={pageT("metrics.namespaces", "Namespaces")}
          value={numberText(graph.metrics.namespaceCount)}
          trend={pageT("metrics.namespacesTrend", "graph storage targets")}
        />
      </section>

      <SurfaceCard className="overflow-hidden p-0">
        <GraphToolbar
          activeLabels={filters.labels}
          facets={graph.facets}
          namespace={filters.namespace}
          namespaces={graph.namespaces}
          query={filters.query}
          relationship={filters.relationship}
          relationships={relationships}
          workspaceCode={filters.workspaceCode}
          onActiveLabelsChange={(labels) => setFilter("labels", labels)}
          onNamespaceChange={(namespace) => setFilter("namespace", namespace)}
          onQueryChange={(query) => setFilter("query", query)}
          onZoomChange={(delta) => setZoom((current) => Math.min(140, Math.max(70, current + delta)))}
          setRelationship={(relationship) => setFilter("relationship", relationship)}
        />

        <div className="grid min-h-[620px] xl:grid-cols-[280px_minmax(0,1fr)]">
          <GraphResultRail
            nodes={graph.resultNodes}
            selectedNodeCode={graph.selectedNode.nodeCode}
            onSelectNode={selectNode}
          />
          <GraphCanvas
            edges={graph.visibleEdges}
            highlightedEdgeCodes={graph.highlightedEdgeCodes}
            highlightedNodeCodes={graph.highlightedNodeCodes}
            nodes={graph.visibleNodes}
            selectedNodeCode={graph.selectedNode.nodeCode}
            zoom={zoom}
            onSelectNode={selectNode}
          />
        </div>
      </SurfaceCard>
    </div>
  );
}

export function KnowledgeGraphPage() {
  return <GraphPage />;
}
