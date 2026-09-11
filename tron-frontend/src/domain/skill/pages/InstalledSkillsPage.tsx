import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { createPortal } from "react-dom";
import {
  CircleAlert,
  Maximize2,
  Plus,
  RefreshCcw,
  RefreshCw,
  Trash2,
  X,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable } from "@/components/data-table";
import { DetailField } from "@/components/detail-field";
import { MarkdownContent } from "@/components/markdown-content";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
  SurfaceCard,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  fetchRegistrySkill,
  fetchRegistrySkills,
  rescanRegistrySkills,
  uninstallRegistrySkill,
  updateRegistrySkills,
} from "@/domain/skill/api";
import type {
  RegistrySkillDetail,
  RegistrySkillRecord,
  SkillSourceType,
  SkillUpdateStatus,
} from "@/domain/skill/model";
import { skillIndexWarningMessage } from "@/domain/skill/utils/ragIndexWarning";

function getSourceTone(sourceType: SkillSourceType) {
  switch (sourceType) {
    case "GitHub":
    case "Marketplace":
      return "info" as const;
    case "Manual":
      return "warning" as const;
    default:
      return "info" as const;
  }
}

function getUpdateTone(updateStatus: SkillUpdateStatus) {
  switch (updateStatus) {
    case "UpToDate":
      return "success" as const;
    case "UpdateAvailable":
      return "warning" as const;
    case "Error":
      return "error" as const;
    case "Manual":
      return "info" as const;
    default:
      return "info" as const;
  }
}

export function InstalledSkillsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const [skills, setSkills] = useState<RegistrySkillRecord[]>([]);
  const [selectedSkillName, setSelectedSkillName] = useState("");
  const [selectedSkill, setSelectedSkill] = useState<RegistrySkillDetail | null>(null);
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [syncing, setSyncing] = useState<"refresh" | "update" | null>(null);
  const [uninstallingSkillName, setUninstallingSkillName] = useState("");
  const [uninstallTarget, setUninstallTarget] = useState<RegistrySkillRecord | null>(null);
  const [skillMarkdownFullscreenOpen, setSkillMarkdownFullscreenOpen] = useState(false);
  const detailRequestIdRef = useRef(0);
  const pagination = useTablePagination(skills);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:installedSkills.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
    }, 300);
    return () => window.clearTimeout(timer);
  }, [searchInput]);

  async function loadRegistry() {
    setLoading(true);
    try {
      const records = await fetchRegistrySkills({
        search: debouncedSearch || undefined,
      });
      setSkills(records);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.load", "Installed skills could not be loaded");
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadRegistry();
  }, [debouncedSearch]);

  useEffect(() => {
    if (!skills.length) {
      setSelectedSkillName("");
      setUninstallTarget(null);
      return;
    }
    if (!skills.some((skill) => skill.skillName === selectedSkillName)) {
      setSelectedSkillName(skills[0].skillName);
    }
    if (
      uninstallTarget
      && !skills.some((skill) => skill.skillName === uninstallTarget.skillName)
    ) {
      setUninstallTarget(null);
    }
  }, [skills, selectedSkillName, uninstallTarget]);

  useEffect(() => {
    if (!selectedSkillName) {
      setSelectedSkill(null);
      return;
    }

    const requestId = detailRequestIdRef.current + 1;
    detailRequestIdRef.current = requestId;
    setDetailLoading(true);
    setSelectedSkill(null);
    fetchRegistrySkill(selectedSkillName)
      .then((detail) => {
        if (detailRequestIdRef.current !== requestId) {
          return;
        }
        setSelectedSkill(detail);
      })
      .catch((error) => {
        if (detailRequestIdRef.current !== requestId) {
          return;
        }
        const message =
          error instanceof Error ? error.message : pageT("errors.detail", "Skill detail could not be loaded");
        toast.error(message);
      })
      .finally(() => {
        if (detailRequestIdRef.current === requestId) {
          setDetailLoading(false);
        }
      });
  }, [selectedSkillName, toast]);

  useEffect(() => {
    if (!skillMarkdownFullscreenOpen) return;

    const originalOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setSkillMarkdownFullscreenOpen(false);
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [skillMarkdownFullscreenOpen]);

  useEffect(() => {
    if (!uninstallTarget) return;

    const originalOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !uninstallingSkillName) {
        setUninstallTarget(null);
      }
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [uninstallTarget, uninstallingSkillName]);

  async function handleRegistrySync(mode: "refresh" | "update") {
    setSyncing(mode);
    try {
      const result = mode === "refresh"
        ? await rescanRegistrySkills()
        : await updateRegistrySkills();
      const warning = skillIndexWarningMessage(result.ragIndex, pageT("warnings.ragIndex", "Skill RAG index completed with warnings"));
      if (mode === "refresh") {
        if (warning) {
          toast.warning(warning);
        } else {
          toast.success(pageT("messages.discovered", "Discovered {{count}} installed skills", { count: formatNumber(result.discoveredCount) }));
        }
      } else if (warning) {
        toast.warning(warning);
      } else {
        toast.success(pageT("messages.updated", "Registry skills updated"));
      }
      await loadRegistry();
      if (selectedSkillName) {
        const detail = await fetchRegistrySkill(selectedSkillName);
        if (detail.skillName === selectedSkillName) {
          setSelectedSkill(detail);
        }
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.registry", "Registry operation failed");
      toast.error(message);
    } finally {
      setSyncing(null);
    }
  }

  async function executeUninstall(skill: RegistrySkillRecord, removeBindings: boolean) {
    setUninstallingSkillName(skill.skillName);
    try {
      const result = await uninstallRegistrySkill(skill.skillName, removeBindings);
      await loadRegistry();
      setSelectedSkill(null);
      setUninstallTarget(null);
      const warning = skillIndexWarningMessage(result.ragIndex, pageT("warnings.ragIndex", "Skill RAG index completed with warnings"));
      if (warning) {
        toast.warning(warning);
      } else if (result.removedBindingsCount > 0) {
        toast.success(
          pageT("messages.uninstalledRemoved", "{{name}} uninstalled and removed from {{count}} workspace binding{{suffix}}", {
            name: skill.skillName,
            count: formatNumber(result.removedBindingsCount),
            suffix: result.removedBindingsCount === 1 ? "" : "s",
          }),
        );
      } else if (result.keptBindingsCount > 0) {
        toast.success(
          pageT("messages.uninstalledKept", "{{name}} uninstalled. {{count}} workspace binding{{suffix}} remain and will show as deleted.", {
            name: skill.skillName,
            count: formatNumber(result.keptBindingsCount),
            suffix: result.keptBindingsCount === 1 ? "" : "s",
          }),
        );
      } else {
        toast.success(pageT("messages.uninstalled", "{{name}} uninstalled", { name: skill.skillName }));
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.uninstall", "Skill uninstall failed");
      toast.error(message);
    } finally {
      setUninstallingSkillName("");
    }
  }

  return (
    <>
      <PageHeader
        title={pageT("title", "Skill Center")}
        actions={(
          <>
            <Button
              size='lg'
              variant="outline"
              disabled={syncing !== null}
              aria-busy={syncing === "update"}
              onClick={() => void handleRegistrySync("update")}
            >
              {syncing === "update" ? pageT("actions.updating", "Updating...") : pageT("actions.update", "Update")}
            </Button>
            <Button size='lg' onClick={() => navigate("/settings/skills/sync")}>
              {pageT("actions.sync", "Sync")}
            </Button>
          </>
        )}
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_420px]">
        <SurfaceCard className="p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
            <div className="min-w-[16rem] flex-1">
              <SearchField
                ariaLabel={pageT("search.ariaLabel", "Filter installed skills")}
                clearLabel={pageT("search.clear", "Clear installed skill search")}
                placeholder={pageT("search.placeholder", "Filter installed skills...")}
                value={searchInput}
                onChange={setSearchInput}
              />
            </div>
            <div className="flex items-center gap-[12px]">
              <Button
                size="lg"
                disabled={syncing !== null}
                aria-busy={syncing === "refresh"}
                onClick={() => void handleRegistrySync("refresh")}
              >
                {syncing === "refresh" ? pageT("actions.refreshing", "Refreshing...") : t("common:actions.refresh")}
              </Button>
              {loading ? <Badge variant="info">{pageT("loading.badge", "Loading")}</Badge> : null}
            </div>
          </div>
          <DataTable<RegistrySkillRecord>
            variant="embedded"
            onRowClick={(row) => setSelectedSkillName(row.skillName)}
            rows={pagination.visibleItems}
            pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "skills")} {...pagination.paginationProps} />}
            rowKey={(row) => row.skillName}
            rowClassName={(row) => (
              row.skillName === selectedSkillName
                ? "bg-[var(--color-surface-container)]"
                : undefined
            )}
            emptyState={(
              <div className="space-y-2">
                <p>{pageT("emptyState.title", "No installed skills match the current filters.")}</p>
                <p className="text-xs">{pageT("emptyState.body", "Use “Sync” to search `skills.sh` and add one to the local registry.")}</p>
              </div>
            )}
            columns={[
              {
                id: "skillName",
                header: pageT("columns.skill", "Skill"),
                render: (row) => (
                  <div
                    className={cn(
                      "rounded-md px-1 py-0.5 text-left font-medium",
                      row.skillName === selectedSkillName
                        ? "text-[var(--color-primary)]"
                        : "text-[var(--color-on-surface)]",
                    )}
                  >
                    <span className="block">{row.skillName}</span>
                    <span className="block max-w-[22rem] truncate text-[12px] text-[var(--color-on-surface-variant)]">
                      {row.sourceRef}
                    </span>
                  </div>
                ),
              },
              {
                id: "version",
                header: pageT("columns.version", "Version"),
                render: (row) => row.declaredVersion ?? pageT("status.unspecified", "Unspecified"),
              },
              {
                id: "sourceType",
                header: pageT("columns.source", "Source"),
                render: (row) => (
                  <Badge variant={getSourceTone(row.sourceType)}>{row.sourceType}</Badge>
                ),
              },
              {
                id: "updateStatus",
                header: pageT("columns.update", "Update"),
                render: (row) => (
                  <Badge variant={getUpdateTone(row.updateStatus)}>{row.updateStatus}</Badge>
                ),
              },
              {
                id: "enabledWorkspaceCount",
                header: pageT("columns.workspaces", "Workspaces"),
                headerClassName: "w-20",
                cellClassName: "w-20",
                render: (row) => formatNumber(row.enabledWorkspaceCount),
              },
              {
                id: "action",
                header: pageT("columns.action", "Action"),
                align: "right",
                render: (row) => {
                  const uninstalling = uninstallingSkillName === row.skillName;
                  return (
                    <div className="flex min-w-[12rem] justify-end">
                      <Button
                        size="sm"
                        type="button"
                        variant="ghost"
                        disabled={uninstalling}
                        aria-busy={uninstalling}
                        onClick={(event) => {
                          event.stopPropagation();
                          setUninstallTarget(row);
                        }}
                      >
                        <Trash2 size={14} aria-hidden="true" />
                        {uninstalling ? pageT("actions.uninstalling", "Uninstalling...") : pageT("actions.uninstall", "Uninstall")}
                      </Button>
                    </div>
                  );
                },
              },
            ]}
          />
        </SurfaceCard>

        <SurfaceCard className="space-y-5">
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
              {pageT("detail.label", "Skill Detail")}
            </p>
            <h2 className="mt-2 text-2xl font-semibold">
              {selectedSkillName || pageT("detail.select", "Select a Skill")}
            </h2>
          </div>

          {!selectedSkillName && !detailLoading ? (
            <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] px-5 py-6 text-sm text-[var(--color-on-surface-variant)]">
              {pageT("detail.empty", "Select an installed skill to review its source, files, and markdown preview.")}
            </div>
          ) : null}

          {detailLoading ? (
            <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] px-5 py-6 text-sm text-[var(--color-on-surface-variant)]">
              {pageT("detail.loading", "Loading skill detail...")}
            </div>
          ) : null}

          {selectedSkill ? (
            <>
              <div className="flex flex-wrap gap-[12px]">
                <Badge variant={getSourceTone(selectedSkill.sourceType)}>
                  {selectedSkill.sourceType}
                </Badge>
                <Badge variant={getUpdateTone(selectedSkill.updateStatus)}>
                  {selectedSkill.updateStatus}
                </Badge>
                <Badge variant="info">{selectedSkill.managedBy === "Cli" ? pageT("detail.managedByCli", "CLI Managed") : pageT("detail.managedByManual", "Manual")}</Badge>
              </div>

              <div className="rounded-2xl border border-[var(--color-outline-variant)] px-4 py-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <DetailField label={pageT("detail.source", "Source")} value={selectedSkill.sourceRef} />
                  <DetailField label={pageT("detail.registryPath", "Registry Path")} value={selectedSkill.registryPath} />
                  <DetailField label={pageT("detail.version", "Version")} value={selectedSkill.declaredVersion ?? pageT("status.unspecified", "Unspecified")} />
                  <DetailField label={pageT("detail.lastChecked", "Last Checked")} value={selectedSkill.lastCheckedAt ? formatDateTime(selectedSkill.lastCheckedAt, { dateStyle: "medium", timeStyle: "short" }) : pageT("detail.notChecked", "Not checked")} />
                  <DetailField label={pageT("detail.lastUpdated", "Last Updated")} value={selectedSkill.lastUpdatedAt ? formatDateTime(selectedSkill.lastUpdatedAt, { dateStyle: "medium", timeStyle: "short" }) : pageT("detail.notChecked", "Not checked")} />
                  <DetailField label={pageT("detail.enabledWorkspaces", "Enabled Workspaces")} value={String(selectedSkill.enabledWorkspaceCount)} />
                </div>
              </div>

              <div>
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                  {pageT("detail.files", "Files")}
                </p>
                <div className="mt-3 flex flex-wrap gap-[12px]">
                  {selectedSkill.files.length ? (
                    selectedSkill.files.map((file) => (
                      <Badge key={file.path} variant={file.isMarkdown ? "info" : "outline"}>
                        {file.path}
                      </Badge>
                    ))
                  ) : (
                    <span className="text-sm text-[var(--color-on-surface-variant)]">
                      {pageT("detail.filesEmpty", "No files discovered.")}
                    </span>
                  )}
                </div>
              </div>

              <div>
                <div className="flex flex-wrap items-center justify-between gap-[12px]">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                    {pageT("detail.markdownPreviewLabel", "SKILL.md Preview")}
                  </p>
                  <Button
                    size="sm"
                    type="button"
                    variant="ghost"
                    aria-label={pageT("aria.openMarkdownFullscreen", "Open skill markdown fullscreen")}
                    onClick={() => setSkillMarkdownFullscreenOpen(true)}
                  >
                    <Maximize2 size={14} aria-hidden="true" />
                    {pageT("detail.fullscreen", "Fullscreen")}
                  </Button>
                </div>
                <div className="mt-2 max-h-[24rem] min-w-0 max-w-full overflow-x-hidden overflow-y-auto overscroll-y-contain rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-4 py-3">
                  <MarkdownContent
                    ariaLabel={pageT("aria.skillMarkdownPreview", "Skill markdown preview")}
                    className="max-w-full pr-1"
                    content={selectedSkill.skillMarkdown}
                  />
                </div>
              </div>
            </>
          ) : null}
        </SurfaceCard>
      </div>
      {uninstallTarget ? (
        <div className="fixed inset-0 z-[85] flex items-center justify-center bg-black/45 p-4 backdrop-blur-sm">
          <div
            className="w-[min(92vw,32rem)] rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={pageT("aria.uninstallConfirmation", "Uninstall skill confirmation")}
          >
            <div className="flex items-start gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div className="mt-0.5 rounded-full bg-[var(--color-error-container)] p-2 text-[var(--color-on-error-container)]">
                <CircleAlert size={18} aria-hidden="true" />
              </div>
              <div className="min-w-0 flex-1">
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">
                  {pageT("uninstallDialog.title", "Uninstall {{name}}", { name: uninstallTarget.skillName })}
                </h2>
                <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                  {uninstallTarget.enabledWorkspaceCount > 0
                    ? pageT("uninstallDialog.usedByWorkspaces", "{{name}} is currently used by {{count}} workspace{{suffix}}. Do you also want to uninstall the corresponding workspace skills?", {
                      name: uninstallTarget.skillName,
                      count: uninstallTarget.enabledWorkspaceCount,
                      suffix: uninstallTarget.enabledWorkspaceCount === 1 ? "" : "s",
                    })
                    : pageT("uninstallDialog.removeLocal", "This will remove the local skill files from Skill Center.")}
                </p>
              </div>
            </div>
            <div className="space-y-3 px-5 py-4">
              {uninstallTarget.enabledWorkspaceCount > 0 ? (
                <>
                  <Button
                    type="button"
                    variant="secondary"
                    className="w-full justify-center"
                    disabled={uninstallingSkillName === uninstallTarget.skillName}
                    aria-busy={uninstallingSkillName === uninstallTarget.skillName}
                    onClick={() => void executeUninstall(uninstallTarget, false)}
                  >
                    {pageT("uninstallDialog.keepWorkspaceSkills", "Keep Workspace Skills")}
                  </Button>
                  <Button
                    type="button"
                    className="w-full justify-center"
                    disabled={uninstallingSkillName === uninstallTarget.skillName}
                    aria-busy={uninstallingSkillName === uninstallTarget.skillName}
                    onClick={() => void executeUninstall(uninstallTarget, true)}
                  >
                    {pageT("uninstallDialog.uninstallWorkspaceToo", "Uninstall Workspace Skills Too")}
                  </Button>
                </>
              ) : (
                <Button
                  type="button"
                  className="w-full justify-center"
                  disabled={uninstallingSkillName === uninstallTarget.skillName}
                  aria-busy={uninstallingSkillName === uninstallTarget.skillName}
                  onClick={() => void executeUninstall(uninstallTarget, false)}
                >
                  {uninstallingSkillName === uninstallTarget.skillName ? pageT("uninstallDialog.uninstalling", "Uninstalling...") : pageT("uninstallDialog.uninstall", "Uninstall")}
                </Button>
              )}
              <Button
                type="button"
                variant="ghost"
                className="w-full justify-center"
                disabled={uninstallingSkillName === uninstallTarget.skillName}
                onClick={() => setUninstallTarget(null)}
              >
                {pageT("uninstallDialog.cancel", "Cancel")}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
      {skillMarkdownFullscreenOpen && selectedSkill ? createPortal(
        <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] min-w-0 max-w-full flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={pageT("aria.skillMarkdownFullscreenPreview", "Skill markdown fullscreen preview")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("detail.markdownFullscreenTitle", "SKILL.md")}</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  {pageT("detail.markdownFullscreenDescription", "Preview the rendered markdown for this skill package.")}
                </p>
              </div>
              <Button
                size="icon"
                type="button"
                variant="ghost"
                aria-label={pageT("aria.closeMarkdownFullscreen", "Close skill markdown fullscreen")}
                onClick={() => setSkillMarkdownFullscreenOpen(false)}
              >
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-y-contain px-5 py-4">
              <MarkdownContent
                ariaLabel={pageT("aria.skillMarkdownFullscreen", "Skill markdown fullscreen")}
                className="max-w-none pr-1"
                content={selectedSkill.skillMarkdown}
              />
            </div>
          </div>
        </div>,
        document.body,
      ) : null}
    </>
  );
}
