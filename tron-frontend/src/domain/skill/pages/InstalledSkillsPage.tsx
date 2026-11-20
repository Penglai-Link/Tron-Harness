import { SkillAgentGrants } from "../components/SkillAgentGrants";
import { useEffect, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { SkillCandidatesPanel } from "../components/SkillCandidatesPanel";
import { Badge } from "@/components/ui/badge";
import { DataTable } from "@/components/data-table";
import { SearchField } from "@/components/search-field";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { TablePagination } from "@/components/table-pagination";
import { useTablePagination } from "@/shared/hooks/useTablePagination";
import { SurfaceCard } from "@/shared";
import { useToast } from "@/components/ui/toast";
import {
  fetchRegistrySkills,
  uninstallRegistrySkill,
  rescanRegistrySkills,
  updateRegistrySkills,
  CAPABILITY_RESOURCES_CHANGED,
  notifyCapabilityResourcesChanged,
} from "@/domain/skill/api";
import type {
  RegistrySkillRecord,
} from "@/domain/skill/model";
import { DisabledResourceReason } from "@/domain/agent/components/InternalAgentSkillField";

export function InstalledSkillsPage() {
  const { t } = useTranslation("pages");
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") === "approvals" || params.has("candidateCode") ? "approvals" : "installed";
  return (
    <Tabs value={tab} onValueChange={(value) => setParams((previous) => {
      const next = new URLSearchParams(previous);
      if (value === "approvals") next.set("tab", "approvals");
      else { next.delete("tab"); next.delete("candidateCode"); }
      return next;
    })}>
      <TabsList variant="line">
        <TabsTrigger value="installed">{t("skillCandidates.installedTab")}</TabsTrigger>
        <TabsTrigger value="approvals">{t("skillCandidates.approvalsTab")}</TabsTrigger>
      </TabsList>
      <TabsContent value="installed"><InstalledSkillList /></TabsContent>
      <TabsContent value="approvals"><SkillCandidatesPanel /></TabsContent>
    </Tabs>
  );
}

function InstalledSkillList() {
  const { t } = useTranslation("pages");
  const navigate = useNavigate();
  const toast = useToast();
  const [skills, setSkills] = useState<RegistrySkillRecord[]>([]);
  const [search, setSearch] = useState("");
  const [grant, setGrant] = useState<RegistrySkillRecord | null>(null);
  const [uninstall, setUninstall] = useState<RegistrySkillRecord | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const pagination = useTablePagination(
    skills.filter((s) =>
      `${s.skillName} ${s.description ?? ""} ${s.sourceRef}`
        .toLowerCase()
        .includes(search.toLowerCase())
    )
  );
  const text = (key: string) => t(`agentSkills.${key}`);
  async function load() {
    setLoading(true);
    try {
      setSkills(await fetchRegistrySkills());
      setError("");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    void load();
    const changed = () => {
      setGrant(null);
      void load();
    };
    window.addEventListener(CAPABILITY_RESOURCES_CHANGED, changed);
    return () =>
      window.removeEventListener(CAPABILITY_RESOURCES_CHANGED, changed);
  }, []);
  async function sync(update: boolean) {
    setBusy(true);
    try {
      const result = await (update
        ? updateRegistrySkills()
        : rescanRegistrySkills());
      if (result.ragIndex?.failedCount)
        toast.warning(result.ragIndex.warnings.join("; "));
      notifyCapabilityResourcesChanged();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    if (!uninstall) return;
    setBusy(true);
    try {
      await uninstallRegistrySkill(uninstall.skillName);
      setUninstall(null);
      notifyCapabilityResourcesChanged();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      {error ? (
        <p role="alert">
          {error}
          <Button variant="outline" onClick={() => void load()}>
            {text("retry")}
          </Button>
        </p>
      ) : null}
      <div>
        <SurfaceCard data-settings-flush className="p-0">
          <div className="flex flex-wrap items-center justify-end gap-3 px-4 pb-4">
            <SearchField
              className="w-full max-w-80"
              ariaLabel={text("search")}
              placeholder={text("search")}
              value={search}
              onChange={setSearch}
            />
            <Button
              variant="outline"
              disabled={busy || loading}
              onClick={() => void sync(false)}
            >
              {t("installedSkills.actions.refresh")}
            </Button>
            <div className="flex items-center gap-3">
              <Button variant="outline" disabled={busy} onClick={() => void sync(true)}>
                {t("installedSkills.actions.update")}
              </Button>
              <Button onClick={() => navigate("/settings/skills/sync")}>
                {t("installedSkills.actions.sync")}
              </Button>
            </div>
          </div>
          <DataTable<RegistrySkillRecord>
            variant="embedded"
            pagination={
              <TablePagination
                itemLabel={t("installedSkills.pagination.itemLabel")}
                {...pagination.paginationProps}
              />
            }
            rows={pagination.visibleItems}
            rowKey={(s) => s.skillName}
            onRowClick={(s) => navigate(`/settings/skills/${encodeURIComponent(s.skillName)}`)}
            emptyState={<p>{loading ? text("loading") : text("empty")}</p>}
            columns={[
              {
                id: "name",
                header: t("installedSkills.columns.skill"),
                render: (s) => (
                  <div>
                    <Link className="block font-medium hover:underline" to={`/settings/skills/${encodeURIComponent(s.skillName)}`} onClick={(event) => event.stopPropagation()}>{s.skillName}</Link>
                    <span className="text-xs text-[var(--color-on-surface-variant)]">
                      {s.sourceRef}
                    </span>
                  </div>
                ),
              },
              {
                id: "source",
                header: t("installedSkills.columns.source"),
                render: (s) => (
                  <Badge variant="outline">
                    {s.providerPackId || s.sourceType}
                  </Badge>
                ),
              },
              {
                id: "status",
                header: text("availability"),
                render: (s) => (
                  <span>
                    <Badge variant={s.available ? "success" : "warning"}>
                      {text(s.available ? "available" : "unavailable")}
                    </Badge>
                    <DisabledResourceReason reason={s.disabledReason} />
                  </span>
                ),
              },
              {
                id: "agents",
                header: text("assignedAgents"),
                render: (s) => s.assignedAgentCount,
              },
              {
                id: "actions",
                header: t("installedSkills.columns.action"),
                render: (s) => (
                  <div className="flex gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy || !s.available}
                      onClick={(e) => {
                        e.stopPropagation();
                        setGrant(s);
                      }}
                    >
                      {text("assignAgents")}
                    </Button>
                    {!s.providerPackId ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={busy}
                        onClick={(e) => {
                          e.stopPropagation();
                          setUninstall(s);
                        }}
                      >
                        {t("installedSkills.actions.uninstall")}
                      </Button>
                    ) : null}
                  </div>
                ),
              },
            ]}
          />
        </SurfaceCard>
      </div>
      {grant ? (
        <SkillAgentGrants
          skill={grant}
          onClose={() => setGrant(null)}
          onSaved={() => {
            setGrant(null);
            notifyCapabilityResourcesChanged();
          }}
        />
      ) : null}
      {uninstall ? (
        <Dialog
          open
          disablePointerDismissal
          onOpenChange={(o) => {
            if (!o && !busy) setUninstall(null);
          }}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {t("installedSkills.actions.uninstall")} {uninstall.skillName}
              </DialogTitle>
              <DialogDescription>{text("uninstallHint")}</DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => setUninstall(null)}
              >
                {text("cancel")}
              </Button>
              <Button disabled={busy} onClick={() => void remove()}>
                {t("installedSkills.actions.uninstall")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}
    </>
  );
}
