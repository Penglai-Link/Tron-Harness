import {
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import { useTranslation } from "react-i18next";

import {
  PageHeader,
  SegmentedTabs,
  useConsoleShellConfig,
  useTablePagination,
  useWorkspaceSpaces,
} from "@/shared";

import { useAuthSession } from "@/domain/auth/hooks";
import { RagCorporaTab } from "@/domain/rag/components/RagCorporaTab";
import { RagDataTab } from "@/domain/rag/components/RagDataTab";
import { RagDiagnosticsTab } from "@/domain/rag/components/RagDiagnosticsTab";
import { RagProfilesTab } from "@/domain/rag/components/RagProfilesTab";
import { RagWorkbenchTab } from "@/domain/rag/components/RagWorkbenchTab";
import {
  sources,
  textUnits,
  traces,
  warnings,
} from "@/domain/rag/mock/data";
import { useRagCorpora } from "@/domain/rag/hooks/useRagCorpora";
import { useRagProfiles } from "@/domain/rag/hooks/useRagProfiles";
import {
  ragTabs,
  type DataView,
  type RagTab,
} from "@/domain/rag/model/RagModel";

export function RagPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const { t } = useTranslation("pages");
  const { session } = useAuthSession();
  const workspaceSpaces = useWorkspaceSpaces();
  const { corpora: corpusRecords } = useRagCorpora();
  const { profiles: profileRecords } = useRagProfiles();
  const [activeTab, setActiveTab] = useState<RagTab>(() => {
    const requestedTab = searchParams.get("tab");
    return ragTabs.some((tab) => tab.value === requestedTab) ? requestedTab as RagTab : "corpora";
  });
  const [dataView, setDataView] = useState<DataView>("sources");
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("All");
  const [traceEnabled, setTraceEnabled] = useState(true);
  const [notice, setNotice] = useState("");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`rag.${key}`, { defaultValue, ...options })
  );
  const translatedRagTabs = useMemo(() => ragTabs.map((tab) => ({
    ...tab,
    label: pageT(`tabs.${tab.value}`, tab.label),
  })), [t]);

  const activeWorkspaceCode = workspaceSpaces.selectedSecondarySpace?.workspaceCode
    ?? workspaceSpaces.selectedPrimarySpace?.workspaceCode
    ?? "TRON_CORE";
  const activeUserCode = session?.user.userCode ?? "USR-AUTH-0001";

  useEffect(() => {
    const requestedTab = searchParams.get("tab");
    const normalizedTab = ragTabs.some((tab) => tab.value === requestedTab) ? requestedTab as RagTab : "corpora";
    if (normalizedTab !== activeTab) {
      setActiveTab(normalizedTab);
    }
  }, [activeTab, searchParams]);

  const filteredCorpora = useMemo(() => corpusRecords.filter((corpus) => {
    const normalizedQuery = query.trim().toLowerCase();
    const matchesQuery = !normalizedQuery
      || corpus.name.toLowerCase().includes(normalizedQuery)
      || corpus.code.toLowerCase().includes(normalizedQuery)
      || corpus.description.toLowerCase().includes(normalizedQuery);
    const matchesStatus = statusFilter === "All" || corpus.status === statusFilter;
    return matchesQuery && matchesStatus;
  }), [corpusRecords, query, statusFilter]);

  const filteredSources = useMemo(() => sources.filter((source) => {
    const normalizedQuery = query.trim().toLowerCase();
    return !normalizedQuery
      || source.code.toLowerCase().includes(normalizedQuery)
      || source.corpus.toLowerCase().includes(normalizedQuery)
      || source.sourceType.toLowerCase().includes(normalizedQuery);
  }), [query]);

  const filteredTextUnits = useMemo(() => textUnits.filter((unit) => {
    const normalizedQuery = query.trim().toLowerCase();
    return !normalizedQuery
      || unit.id.toLowerCase().includes(normalizedQuery)
      || unit.preview.toLowerCase().includes(normalizedQuery)
      || unit.source.toLowerCase().includes(normalizedQuery);
  }), [query]);

  const filteredWarnings = useMemo(() => warnings.filter((warning) => {
    const normalizedQuery = query.trim().toLowerCase();
    return !normalizedQuery
      || warning.message.toLowerCase().includes(normalizedQuery)
      || warning.source.toLowerCase().includes(normalizedQuery)
      || warning.stage.toLowerCase().includes(normalizedQuery);
  }), [query]);

  const filteredTraces = useMemo(() => traces.filter((trace) => {
    const normalizedQuery = query.trim().toLowerCase();
    return !normalizedQuery
      || trace.id.toLowerCase().includes(normalizedQuery)
      || trace.query.toLowerCase().includes(normalizedQuery)
      || trace.profile.toLowerCase().includes(normalizedQuery);
  }), [query]);

  const corpusPagination = useTablePagination(filteredCorpora, 10);
  const sourcePagination = useTablePagination(filteredSources, 10);
  const textUnitPagination = useTablePagination(filteredTextUnits, 10);
  const warningPagination = useTablePagination(filteredWarnings, 10);
  const tracePagination = useTablePagination(filteredTraces, 10);

  useConsoleShellConfig({ searchPlaceholder: pageT("search.placeholder", "Search RAG corpora, profiles, and sources...") });

  return (
    <>
      <PageHeader title={pageT("title", "RAG Management")} />

      <div className="mb-5 flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
        <SegmentedTabs
          items={translatedRagTabs}
          value={activeTab}
          onValueChange={(value) => {
            setActiveTab(value as RagTab);
            const nextParams = new URLSearchParams(searchParams);
            if (value === "corpora") {
              nextParams.delete("tab");
            } else {
              nextParams.set("tab", value);
            }
            setSearchParams(nextParams, { replace: true });
            setNotice(pageT("messages.tabChanged", "RAG tab switched to {{label}}.", {
              label: translatedRagTabs.find((tab) => tab.value === value)?.label ?? value,
            }));
          }}
        />
      </div>

      {activeTab === "corpora" ? (
        <RagCorporaTab
          corpusPagination={corpusPagination}
          filteredCount={filteredCorpora.length}
          query={query}
          statusFilter={statusFilter}
          onCreateCorpus={() => navigate("/rag/corpora/new")}
          onEditCorpus={(corpus) => navigate(`/rag/corpora/${encodeURIComponent(corpus.code)}/edit`)}
          onQueryChange={setQuery}
          onStatusFilterChange={setStatusFilter}
        />
      ) : null}

      {activeTab === "profiles" ? (
        <RagProfilesTab profiles={profileRecords} />
      ) : null}

      {activeTab === "workbench" ? (
        <RagWorkbenchTab
          corpora={corpusRecords}
          profiles={profileRecords}
          traceEnabled={traceEnabled}
          userCode={activeUserCode}
          workspaceCode={activeWorkspaceCode}
          onNotice={setNotice}
          onTraceEnabledChange={setTraceEnabled}
        />
      ) : null}

      {activeTab === "data" ? (
        <RagDataTab
          dataView={dataView}
          query={query}
          sourcePagination={sourcePagination}
          textUnitPagination={textUnitPagination}
          tracePagination={tracePagination}
          warningPagination={warningPagination}
          onDataViewChange={setDataView}
          onQueryChange={setQuery}
          onSelectSource={(source) => {
            setNotice(pageT("messages.sourceSelected", "Source selected: {{code}}.", { code: source.code }));
          }}
        />
      ) : null}

      {activeTab === "diagnostics" ? (
        <RagDiagnosticsTab onNotice={setNotice} />
      ) : null}

      <p className="sr-only" aria-live="polite">{notice}</p>
    </>
  );
}
