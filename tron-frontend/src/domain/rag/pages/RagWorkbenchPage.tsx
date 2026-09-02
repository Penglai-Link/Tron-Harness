import {
  useMemo,
  useState,
} from "react";
import {
  useNavigate,
  useParams,
} from "react-router";
import { useTranslation } from "react-i18next";

import {
  PageHeader,
  useTabPanelConfig,
  useWorkspaceSpaces,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import { useAuthSession } from "@/domain/auth/hooks";
import { RagWorkbenchTab } from "@/domain/rag/components/RagWorkbenchTab";
import { useRagCorpora } from "@/domain/rag/hooks/useRagCorpora";
import { useRagProfiles } from "@/domain/rag/hooks/useRagProfiles";
import type { IngestProfileRecord } from "@/domain/rag/model/RagModel";
import { resolveCorpusIngestProfiles } from "@/domain/rag/model/ragProfileBindings";

function useRagWorkbenchPageData() {
  const { session } = useAuthSession();
  const workspaceSpaces = useWorkspaceSpaces();
  const { corpora } = useRagCorpora();
  const { profiles } = useRagProfiles();
  const ingestProfileRecords = useMemo(
    () => profiles.filter((profile): profile is IngestProfileRecord => profile.type === "Ingest"),
    [profiles],
  );
  const corporaWithProfileBindings = useMemo(
    () => resolveCorpusIngestProfiles(corpora, ingestProfileRecords),
    [corpora, ingestProfileRecords],
  );
  const workspaceCode = workspaceSpaces.selectedSecondarySpace?.workspaceCode
    ?? workspaceSpaces.selectedPrimarySpace?.workspaceCode
    ?? "TRON_CORE";
  const userCode = session?.user.userCode ?? "USR-AUTH-0001";
  return {
    corpora: corporaWithProfileBindings,
    profiles,
    userCode,
    workspaceCode,
  };
}

export function RagIngestTestPage() {
  const navigate = useNavigate();
  const { corpusCode = "" } = useParams();
  const { t } = useTranslation("pages");
  const [notice, setNotice] = useState("");
  const [traceEnabled, setTraceEnabled] = useState(true);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`rag.${key}`, { defaultValue, ...options })
  );
  const data = useRagWorkbenchPageData();
  const selectedCorpus = data.corpora.find((corpus) => corpus.code === corpusCode);
  const title = pageT("workbench.testPages.ingestTitle", "Ingest test");

  useTabPanelConfig({ searchPlaceholder: pageT("search.placeholder", "Search RAG corpora and retrieval profiles...") });

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "RAG", href: "/rag" }, { label: selectedCorpus?.name ?? (corpusCode || title) }]}
        title={title}
        actions={<Button variant="secondary" onClick={() => navigate("/rag")}>{pageT("actions.backToList", "Back to list")}</Button>}
      />
      <RagWorkbenchTab
        corpora={data.corpora}
        hidePanelSwitcher
        initialCorpusId={corpusCode}
        initialIngestProfileId={selectedCorpus?.ingestProfile || undefined}
        initialPanel="ingest"
        lockIngestContext
        profiles={data.profiles}
        traceEnabled={traceEnabled}
        userCode={data.userCode}
        workspaceCode={data.workspaceCode}
        onNotice={setNotice}
        onTraceEnabledChange={setTraceEnabled}
      />
      <p className="sr-only" aria-live="polite">{notice}</p>
    </>
  );
}

export function RagRetrievalTestPage() {
  const navigate = useNavigate();
  const { profileId = "" } = useParams();
  const { t } = useTranslation("pages");
  const [notice, setNotice] = useState("");
  const [traceEnabled, setTraceEnabled] = useState(true);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`rag.${key}`, { defaultValue, ...options })
  );
  const data = useRagWorkbenchPageData();
  const title = pageT("workbench.testPages.retrievalTitle", "Retrieval test");

  useTabPanelConfig({ searchPlaceholder: pageT("search.placeholder", "Search RAG corpora and retrieval profiles...") });

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "RAG", href: "/rag?section=retrieval" }, { label: profileId || title }]}
        title={title}
        actions={<Button variant="secondary" onClick={() => navigate("/rag?section=retrieval")}>{pageT("actions.backToList", "Back to list")}</Button>}
      />
      <RagWorkbenchTab
        corpora={data.corpora}
        hidePanelSwitcher
        initialPanel="retrieval"
        initialRetrievalProfileId={profileId}
        lockRetrievalContext
        profiles={data.profiles}
        traceEnabled={traceEnabled}
        userCode={data.userCode}
        workspaceCode={data.workspaceCode}
        onNotice={setNotice}
        onTraceEnabledChange={setTraceEnabled}
      />
      <p className="sr-only" aria-live="polite">{notice}</p>
    </>
  );
}
