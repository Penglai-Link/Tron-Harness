import { useSearchParams } from "react-router";
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/shared";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { LoopDesigner } from "../components/LoopDesigner";
import { WorkspaceAgentsPage } from "./WorkspaceAgentsPage";
import { PlatformSystemAgentsSection } from "../components/PlatformSystemAgentsSection";
import { INTERNAL_AGENT_REGISTRY_CHANGED_EVENT } from "../model";
import { DudeDefinitions } from "@/domain/dude/components/DudeDefinitions";
import { InfoTooltip } from "@/components/info-tooltip";

export function PlatformAgentsPage() {
  const { t } = useTranslation("pages");
  const queryClient = useQueryClient();
  const [search, setSearch] = useSearchParams();
  const tab = search.get("tab") ?? "agents";
  useEffect(() => {
    const refresh = () => { void queryClient.invalidateQueries({ queryKey: ["knowledge-base-agents"] }); };
    window.addEventListener(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, refresh);
    return () => window.removeEventListener(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, refresh);
  }, [queryClient]);
  return <div className="space-y-6">
    <PageHeader title={t("settingsAgents.title")} />
    <Tabs value={tab} onValueChange={value => { if (typeof value === "string") setSearch({ tab: value }); }}>
      <TabsList>
        <TabsTrigger value="agents">{t("settingsAgents.title")}</TabsTrigger>
        <TabsTrigger value="loops">{t("agentLoops.title")}</TabsTrigger>
      </TabsList>
      <TabsContent keepMounted value="agents" className="mt-4 space-y-6">
        <DudeDefinitions
          embedded
          title={t("dude.category")}
          titleHelp={t("dude.categoryHelp")}
          titleHelpAriaLabel={t("dude.categoryHelpAria")}
        />
        <section aria-label={t("platformData.agentDefinitions")}>
          <WorkspaceAgentsPage category="internal" scope="platform" />
        </section>
        <PlatformSystemAgentsSection />
      </TabsContent>
      <TabsContent keepMounted value="loops" className="mt-5"><LoopDesigner /></TabsContent>
    </Tabs>
  </div>;
}
