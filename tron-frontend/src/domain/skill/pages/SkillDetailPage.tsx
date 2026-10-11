import { useEffect, useState } from "react";
import { ArrowLeft, RefreshCw } from "lucide-react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { MarkdownContent } from "@/components/markdown-content";
import { PageHeader, SurfaceCard } from "@/shared";
import { DisabledResourceReason } from "@/domain/agent/components/InternalAgentSkillField";
import { CAPABILITY_RESOURCES_CHANGED, fetchRegistrySkill } from "../api";
import type { RegistrySkillDetail } from "../model";

export function SkillDetailPage() {
  const { skillName } = useParams<{ skillName: string }>();
  const { t } = useTranslation("pages");
  const [detail, setDetail] = useState<RegistrySkillDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const reload = () => setRevision((value) => value + 1);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    setDetail(null);
    if (!skillName) {
      setLoading(false);
      return;
    }
    void fetchRegistrySkill(skillName).then((record) => {
      if (active) setDetail(record);
    }).catch((reason: unknown) => {
      if (active) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [skillName, revision]);

  useEffect(() => {
    window.addEventListener(CAPABILITY_RESOURCES_CHANGED, reload);
    return () => window.removeEventListener(CAPABILITY_RESOURCES_CHANGED, reload);
  }, []);

  return <div className="min-w-0 space-y-5">
    <PageHeader title={t("installedSkills.detail.title")} breadcrumbs={[
      { label: t("installedSkills.title"), href: "/settings/skills" },
      { label: skillName ?? t("installedSkills.detail.title") },
    ]} actions={<div className="flex gap-2">
      <Link className={buttonVariants({ variant: "outline", size: "sm" })} to="/settings/skills"><ArrowLeft size={15} />{t("installedSkills.detail.backToSkills")}</Link>
      <Button variant="outline" size="sm" disabled={loading} onClick={reload}><RefreshCw size={15} className={loading ? "animate-spin" : ""} />{t("installedSkills.actions.refresh")}</Button>
    </div>} />
    {loading ? <p role="status">{t("agentSkills.loading")}</p> : error ? <SurfaceCard role="alert">
      <p className="text-sm text-[var(--color-error)]">{error}</p>
      <Button className="mt-3" variant="outline" onClick={reload}>{t("agentSkills.retry")}</Button>
    </SurfaceCard> : detail ? <>
      <SurfaceCard className="space-y-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0"><h1 className="break-words text-xl font-semibold">{detail.skillName}</h1><p className="mt-2 break-all text-sm text-[var(--color-on-surface-variant)]">{detail.sourceRef}</p></div>
          <div className="flex items-center gap-2"><Badge variant={detail.available ? "success" : "warning"}>{t(`agentSkills.${detail.available ? "available" : "unavailable"}`)}</Badge><DisabledResourceReason reason={detail.disabledReason} /></div>
        </div>
        <p className="text-sm">{t("agentSkills.assignedAgents")}: {detail.assignedAgentCount}</p>
        <div className="flex flex-wrap gap-2">{detail.files.map((file) => <Badge key={file.path} variant="outline" className="max-w-full whitespace-normal break-all">{file.path}</Badge>)}</div>
      </SurfaceCard>
      <SurfaceCard className="min-w-0"><MarkdownContent content={detail.skillMarkdown} ariaLabel={`${detail.skillName} · SKILL.md`} /></SurfaceCard>
    </> : <p>{t("agentSkills.empty")}</p>}
  </div>;
}
