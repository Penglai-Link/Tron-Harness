import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { listRagCorpora, listRagTasks } from "../api/ragApi";
import { RagDefinitionTab } from "../components/RagDefinitionTab";
import { RagIngestionTab } from "../components/RagIngestionTab";
import { RagRetrievalTab } from "../components/RagRetrievalTab";
import { useAuthStore } from "@/store/authStore";
import { useToast } from "@/components/ui/toast";
import { useRagErrorToast } from "../hooks/useRagErrorToast";

export function RagPage() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [params, setParams] = useSearchParams();
  const tab = ["definition", "ingestion", "retrieval"].includes(params.get("tab") ?? "") ? params.get("tab")! : "definition";
  const selected = params.get("corpus") ?? "";
  const stateFilter = params.get("state") ?? "";
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const client = useQueryClient();
  const corpora = useQuery({ queryKey: ["rag-corpora", user], queryFn: ({ signal }) => listRagCorpora(signal) });
  useRagErrorToast(corpora.error?.message);
  const tasks = useQuery({ queryKey: ["rag-tasks", user], queryFn: ({ signal }) => listRagTasks(signal), refetchInterval: tab === "ingestion" ? 3000 : false });
  const refresh = () => { void client.invalidateQueries({ queryKey: ["rag-corpora", user] }); void client.invalidateQueries({ queryKey: ["rag-tasks", user] }); };
  const switchTab = (value: string) => { const next = new URLSearchParams(params); next.set("tab", value); setParams(next); };
  const selectCorpus = (id: string) => setParams((current) => {
    const next = new URLSearchParams(current);
    if (id) next.set("corpus", id); else next.delete("corpus");
    return next;
  });
  const openIngestion = (id: string) => {
    const count = corpora.data?.find((corpus) => corpus.id === id)?.counts.rebuildSourceCount ?? 0;
    if (count > 0) toast.info(t("ragUnified.rebuildSelectionHelp", { count }));
    setParams((current) => {
      const next = new URLSearchParams(current);
      next.set("tab", "ingestion");
      next.set("corpus", id);
      next.delete("state");
      return next;
    });
  };
  const selectState = (state: string) => setParams((current) => {
    const next = new URLSearchParams(current);
    if (state) next.set("state", state); else next.delete("state");
    return next;
  });
  return <Tabs value={tab} onValueChange={(value) => switchTab(String(value))}>
    <div className="flex items-center justify-between gap-4">
      <TabsList aria-label={t("ragUnified.tabsLabel")}>
        <TabsTrigger value="definition">{t("ragUnified.definition")}</TabsTrigger>
        <TabsTrigger value="ingestion">{t("ragUnified.ingestion")}</TabsTrigger>
        <TabsTrigger value="retrieval">{t("ragUnified.retrieval")}</TabsTrigger>
      </TabsList>
      {tab !== "ingestion" ? <Button variant="outline" onClick={refresh} disabled={corpora.isFetching}>{t("ragUnified.refresh")}</Button> : null}
    </div>
    {corpora.isPending ? <p role="status">{t("ragUnified.loading")}</p> : null}
    <TabsContent value="definition"><RagDefinitionTab corpora={corpora.data ?? []} onSaved={refresh} onIngestion={openIngestion} /></TabsContent>
    <TabsContent value="ingestion"><RagIngestionTab key={user} corpora={corpora.data ?? []} tasks={tasks.data ?? []} selected={selected} onSelect={selectCorpus} stateFilter={stateFilter} onSelectState={selectState} onChanged={refresh} onRefresh={refresh} refreshing={corpora.isFetching || tasks.isFetching} error={tasks.error?.message} /></TabsContent>
    <TabsContent value="retrieval"><RagRetrievalTab key={user} corpora={corpora.data ?? []} selected={selected} onSelect={selectCorpus} /></TabsContent>
  </Tabs>;
}
