import { usePlatformDataNavigate } from "@/domain/settings/components/PlatformWorkspaceScope";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  useSearchParams,
} from "react-router";
import { useTranslation } from "react-i18next";

import {
  PageHeader,
  useTabPanelConfig,
  useTablePagination,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  deleteRagCorpus,
  deleteRagProfile,
} from "@/domain/rag/api/ragManagementApi";
import { RagCorporaTab } from "@/domain/rag/components/RagCorporaTab";
import { RagProfilesTab } from "@/domain/rag/components/RagProfilesTab";
import {
  deleteRagMockCorpus,
  deleteRagMockProfile,
} from "@/domain/rag/mock/data";
import { useRagCorpora } from "@/domain/rag/hooks/useRagCorpora";
import { useRagProfiles } from "@/domain/rag/hooks/useRagProfiles";
import {
  type CorpusRecord,
  type IngestProfileRecord,
  type RetrievalProfileRecord,
} from "@/domain/rag/model/RagModel";
import { resolveCorpusIngestProfiles } from "@/domain/rag/model/ragProfileBindings";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function RagPage() {
  const navigate = usePlatformDataNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const { t } = useTranslation("pages");
  const toast = useToast();
  const dataSource = getServiceDataSource("rag");
  const retrievalSectionRef = useRef<HTMLDivElement | null>(null);
  const { corpora: corpusRecords, refresh: refreshCorpora } = useRagCorpora();
  const { profiles: profileRecords, refresh: refreshProfiles } = useRagProfiles();
  const ingestProfileRecords = useMemo(
    () => profileRecords.filter((profile): profile is IngestProfileRecord => profile.type === "Ingest"),
    [profileRecords],
  );
  const corpusRecordsWithProfileBindings = useMemo(
    () => resolveCorpusIngestProfiles(corpusRecords, ingestProfileRecords),
    [corpusRecords, ingestProfileRecords],
  );
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("All");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`rag.${key}`, { defaultValue, ...options })
  );

  function retrievalProfileReferencesCorpus(profile: RetrievalProfileRecord, corpusCode: string) {
    return profile.primaryCorpusId === corpusCode
      || profile.corpus === corpusCode
      || profile.corpusIds.includes(corpusCode)
      || profile.config.routesConfig.some((route) => route.corpusId === corpusCode);
  }

  async function handleDeleteIngestProfile(corpus: CorpusRecord) {
    const profileId = corpus.ingestProfile.trim();
    if (!profileId) return;
    try {
      if (dataSource === "api") {
        await deleteRagProfile(profileId);
      } else {
        deleteRagMockProfile(profileId);
      }
      toast.success(pageT("messages.ingestProfileDeleted", "Ingest profile {{id}} deleted.", { id: profileId }));
      refreshProfiles();
      refreshCorpora();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.deleteIngestProfile", "Ingest profile could not be deleted."));
    }
  }

  async function handleDeleteCorpus(corpus: CorpusRecord) {
    const retrievalBindings = profileRecords.filter((profile): profile is RetrievalProfileRecord => (
      profile.type === "Retrieval" && retrievalProfileReferencesCorpus(profile, corpus.code)
    ));
    if (retrievalBindings.length > 0) {
      toast.error(pageT(
        "errors.deleteCorpusRetrievalBinding",
        "This corpus is bound to retrieval profile {{id}}. Remove the retrieval binding before deleting it.",
        { id: retrievalBindings[0].id },
      ));
      return;
    }
    try {
      const result = dataSource === "api"
        ? await deleteRagCorpus(corpus.code)
        : deleteRagMockCorpus(corpus.code);
      const deletedIngestCount = result.deletedIngestProfileIds?.length ?? 0;
      toast.success(pageT(
        "messages.corpusDeleted",
        "Corpus {{id}} deleted. {{count}} ingest profile(s) removed.",
        { count: deletedIngestCount, id: corpus.code },
      ));
      refreshProfiles();
      refreshCorpora();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.deleteCorpus", "Corpus could not be deleted."));
    }
  }

  async function handleDeleteRetrievalProfile(profile: RetrievalProfileRecord) {
    try {
      if (dataSource === "api") {
        await deleteRagProfile(profile.id);
      } else {
        deleteRagMockProfile(profile.id);
      }
      toast.success(pageT("messages.retrievalProfileDeleted", "Retrieval profile {{id}} deleted.", { id: profile.id }));
      refreshProfiles();
      refreshCorpora();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.deleteRetrievalProfile", "Retrieval profile could not be deleted."));
    }
  }

  useEffect(() => {
    const requestedTab = searchParams.get("tab");
    if (requestedTab) {
      const nextParams = new URLSearchParams(searchParams);
      nextParams.delete("tab");
      if (requestedTab === "profiles") {
        nextParams.set("section", "retrieval");
      }
      setSearchParams(nextParams, { replace: true });
      return;
    }
    if (searchParams.get("section") === "retrieval") {
      window.requestAnimationFrame(() => {
        retrievalSectionRef.current?.scrollIntoView({ block: "start" });
      });
    }
  }, [searchParams, setSearchParams]);

  const filteredCorpora = useMemo(() => corpusRecordsWithProfileBindings.filter((corpus) => {
    const normalizedQuery = query.trim().toLowerCase();
    const matchesQuery = !normalizedQuery
      || corpus.name.toLowerCase().includes(normalizedQuery)
      || corpus.code.toLowerCase().includes(normalizedQuery)
      || corpus.description.toLowerCase().includes(normalizedQuery);
    const matchesStatus = statusFilter === "All" || corpus.status === statusFilter;
    return matchesQuery && matchesStatus;
  }), [corpusRecordsWithProfileBindings, query, statusFilter]);

  const corpusPagination = useTablePagination(filteredCorpora, 10);

  useTabPanelConfig({ searchPlaceholder: pageT("search.placeholder", "Search RAG corpora and retrieval profiles...") });

  return (
    <>
      <div className="grid gap-6">
        <RagCorporaTab
          corpusPagination={corpusPagination}
          query={query}
          statusFilter={statusFilter}
          onCreateCorpus={() => navigate("/rag/corpora/new")}
          onDeleteCorpus={handleDeleteCorpus}
          onDeleteIngestProfile={handleDeleteIngestProfile}
          onEditCorpus={(corpus) => navigate(`/rag/corpora/${encodeURIComponent(corpus.code)}/edit`)}
          onTestIngest={(corpus) => navigate(`/rag/corpora/${encodeURIComponent(corpus.code)}/ingest-test`)}
          onQueryChange={setQuery}
          onStatusFilterChange={setStatusFilter}
        />

        <div ref={retrievalSectionRef} id="rag-retrieval-profiles">
          <RagProfilesTab
            profiles={profileRecords}
            onDeleteProfile={handleDeleteRetrievalProfile}
            onTestProfile={(profile) => navigate(`/rag/profiles/retrieval/${encodeURIComponent(profile.id)}/test`)}
          />
        </div>
      </div>

    </>
  );
}
