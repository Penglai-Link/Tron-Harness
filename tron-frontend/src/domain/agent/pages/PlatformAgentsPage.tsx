import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/shared";
import { WorkspaceAgentsPage } from "./WorkspaceAgentsPage";
import { PlatformSystemAgentsSection } from "../components/PlatformSystemAgentsSection";
import { INTERNAL_AGENT_REGISTRY_CHANGED_EVENT } from "../model";
import { DudeDefinitions } from "@/domain/dude/components/DudeDefinitions";

export function PlatformAgentsPage() {
  const { t } = useTranslation("pages");
  const queryClient = useQueryClient();
  useEffect(() => {
    const refresh = () => { void queryClient.invalidateQueries({ queryKey: ["knowledge-base-agents"] }); };
    window.addEventListener(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, refresh);
    return () => window.removeEventListener(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT, refresh);
  }, [queryClient]);
  return <div className="space-y-6">
    <PageHeader title={t("settingsAgents.title")} />
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
  </div>;
}
