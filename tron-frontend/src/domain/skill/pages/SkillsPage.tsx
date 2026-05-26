import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  CircleAlert,
  Link2,
  Maximize2,
  Package,
  RefreshCcw,
  Unlink2,
  X,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  DetailField,
  InspectorPanel,
  MarkdownContent,
  PageHeader,
  SearchField,
  SurfaceCard,
  TablePagination,
  useConsoleShellConfig,
  useLocaleFormatters,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  bindWorkspaceSkill,
  fetchWorkspaceSkill,
  fetchWorkspaceSkills,
  refreshWorkspaceSkills,
  unbindWorkspaceSkill,
} from "@/domain/skill/api";
import type {
  SkillRegistryState,
  SkillSourceType,
  SkillUpdateStatus,
  SkillWorkspaceSourceKind,
  WorkspaceSkillDetail,
  WorkspaceSkillRecord,
} from "@/domain/skill/model";

function getWorkspaceSourceTone(sourceKind: SkillWorkspaceSourceKind) {
  switch (sourceKind) {
    case "Local":
      return "accent" as const;
    case "Inherited":
      return "neutral" as const;
    case "Override":
      return "warning" as const;
    default:
      return "neutral" as const;
  }
}

function getRegistryStateTone(registryState: SkillRegistryState) {
  switch (registryState) {
    case "Deleted":
      return "error" as const;
    default:
      return "success" as const;
  }
}

function canUnbindSkill(sourceKind: SkillWorkspaceSourceKind) {
  return sourceKind === "Local" || sourceKind === "Override";
}

function getSourceTone(sourceType: SkillSourceType) {
  switch (sourceType) {
    case "GitHub":
    case "Marketplace":
      return "accent" as const;
    case "Manual":
      return "warning" as const;
    default:
      return "neutral" as const;
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
      return "accent" as const;
    default:
      return "neutral" as const;
  }
}

function BindingSwitch({
  checked,
  disabled,
  loading,
  onToggle,
}: {
  checked: boolean;
  disabled: boolean;
  loading: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      aria-busy={loading}
      aria-checked={checked}
      className={cn(
        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-60",
        checked ? "bg-[var(--color-primary)]" : "bg-[var(--color-surface-container-highest)]",
      )}
      disabled={disabled}
      role="switch"
      type="button"
      onClick={onToggle}
    >
      <span className="sr-only">{checked ? "Disable skill in workspace" : "Enable skill in workspace"}</span>
      <span
        className={cn(
          "absolute size-5 rounded-full bg-[var(--color-elevated-surface-strong)] shadow transition-transform",
          checked ? "translate-x-5" : "translate-x-0.5",
        )}
      />
    </button>
  );
}

export function SkillsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode =
    selectedSecondarySpace?.workspaceCode
    ?? selectedPrimarySpace?.workspaceCode
    ?? "";
  const activeWorkspaceLabel =
    selectedSecondarySpace?.label
    ?? selectedPrimarySpace?.name
    ?? t("pages:skills.workspace.unselected", { defaultValue: "No workspace selected" });
  const [skills, setSkills] = useState<WorkspaceSkillRecord[]>([]);
  const [selectedSkillName, setSelectedSkillName] = useState("");
  const [selectedSkill, setSelectedSkill] = useState<WorkspaceSkillDetail | null>(null);
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [bindingSkillName, setBindingSkillName] = useState("");
  const [skillMarkdownFullscreenOpen, setSkillMarkdownFullscreenOpen] = useState(false);
  const detailRequestIdRef = useRef(0);
  const pagination = useTablePagination(skills);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:skills.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
    }, 300);
    return () => window.clearTimeout(timer);
  }, [searchInput]);

  async function loadWorkspaceSkills() {
    if (!activeWorkspaceCode) {
      setSkills([]);
      return;
    }
    setLoading(true);
    try {
      const records = await fetchWorkspaceSkills(activeWorkspaceCode, {
        search: debouncedSearch || undefined,
      });
      setSkills(records);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.load", "Workspace skills could not be loaded");
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadWorkspaceSkills();
  }, [activeWorkspaceCode, debouncedSearch]);

  useEffect(() => {
    if (!skills.length) {
      setSelectedSkillName("");
      return;
    }
    if (!skills.some((skill) => skill.skillName === selectedSkillName)) {
      setSelectedSkillName(skills[0].skillName);
    }
  }, [skills, selectedSkillName]);

  useEffect(() => {
    if (!selectedSkillName || !activeWorkspaceCode) {
      setSelectedSkill(null);
      return;
    }

    const requestId = detailRequestIdRef.current + 1;
    detailRequestIdRef.current = requestId;
    setDetailLoading(true);
    setSelectedSkill(null);
    fetchWorkspaceSkill(activeWorkspaceCode, selectedSkillName)
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
  }, [activeWorkspaceCode, selectedSkillName, toast]);

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

  async function handleRefresh() {
    if (!activeWorkspaceCode) {
      toast.error(pageT("errors.selectWorkspaceRefresh", "Select a workspace in the shell header before refreshing skills"));
      return;
    }

    setRefreshing(true);
    try {
      await refreshWorkspaceSkills(activeWorkspaceCode);
      toast.success(pageT("messages.refreshed", "Workspace skills refreshed"));
      await loadWorkspaceSkills();
      if (selectedSkillName) {
        const detail = await fetchWorkspaceSkill(activeWorkspaceCode, selectedSkillName);
        if (detail.skillName === selectedSkillName) {
          setSelectedSkill(detail);
        }
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.refresh", "Workspace refresh failed");
      toast.error(message);
    } finally {
      setRefreshing(false);
    }
  }

  async function handleToggleBinding(skill: WorkspaceSkillRecord) {
    if (!activeWorkspaceCode) {
      toast.error(pageT("errors.selectWorkspaceEnable", "Select a workspace in the shell header before enabling skills"));
      return;
    }

    setBindingSkillName(skill.skillName);
    try {
      const desiredEnabled = !skill.enabled;
      const shouldRevertInherited =
        skill.sourceKind === "Override" && skill.shadowedEnabled === desiredEnabled;
      const shouldRemoveLocal =
        skill.sourceKind === "Local" && !desiredEnabled;

      if (shouldRevertInherited || shouldRemoveLocal) {
        await unbindWorkspaceSkill(activeWorkspaceCode, skill.skillName);
        toast.success(
          shouldRevertInherited
            ? pageT("messages.reverted", "{{name}} reverted to inherited setting in {{workspace}}", { name: skill.skillName, workspace: activeWorkspaceCode })
            : pageT("messages.removed", "{{name}} removed from {{workspace}}", { name: skill.skillName, workspace: activeWorkspaceCode }),
        );
      } else {
        await bindWorkspaceSkill(activeWorkspaceCode, skill.skillName, desiredEnabled);
        toast.success(
          desiredEnabled
            ? pageT("messages.enabled", "{{name}} enabled in {{workspace}}", { name: skill.skillName, workspace: activeWorkspaceCode })
            : pageT("messages.disabled", "{{name}} disabled in {{workspace}}", { name: skill.skillName, workspace: activeWorkspaceCode }),
        );
      }
      await loadWorkspaceSkills();
      if (selectedSkillName === skill.skillName) {
        const detail = await fetchWorkspaceSkill(activeWorkspaceCode, skill.skillName);
        setSelectedSkill(detail);
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.updateBinding", "Workspace binding could not be updated");
      toast.error(message);
    } finally {
      setBindingSkillName("");
    }
  }

  async function handleUnbind(skill: WorkspaceSkillRecord) {
    if (!activeWorkspaceCode) {
      toast.error(pageT("errors.selectWorkspaceUnbind", "Select a workspace in the shell header before unbinding skills"));
      return;
    }
    if (!canUnbindSkill(skill.sourceKind)) {
      toast.info(pageT("messages.inheritedInfo", "Inherited skills cannot be unbound from this workspace"));
      return;
    }

    setBindingSkillName(skill.skillName);
    try {
      await unbindWorkspaceSkill(activeWorkspaceCode, skill.skillName);
      toast.success(pageT("messages.unbound", "{{name}} unbound from {{workspace}}", { name: skill.skillName, workspace: activeWorkspaceCode }));
      await loadWorkspaceSkills();
      if (selectedSkillName === skill.skillName) {
        const detail = await fetchWorkspaceSkill(activeWorkspaceCode, skill.skillName).catch(() => null);
        setSelectedSkill(detail);
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : pageT("errors.removeBinding", "Workspace binding could not be removed");
      toast.error(message);
    } finally {
      setBindingSkillName("");
    }
  }

  const rightPanel = (
    <InspectorPanel title={pageT("detail.label", "Skill Detail")}>
      <div>
        <h2 className="mt-2 text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
          {(selectedSkill?.skillName ?? selectedSkillName) || pageT("detail.select", "Select a Skill")}
        </h2>
      </div>

      {detailLoading ? (
        <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
          {pageT("detail.loading", "Loading skill detail...")}
        </div>
      ) : !selectedSkill ? (
        <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
          {pageT("detail.empty", "No bound skill is selected.")}
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            {selectedSkill.sourceType ? (
              <Badge tone={getSourceTone(selectedSkill.sourceType)}>
                {selectedSkill.sourceType}
              </Badge>
            ) : null}
            {selectedSkill.updateStatus ? (
              <Badge tone={getUpdateTone(selectedSkill.updateStatus)}>
                {selectedSkill.updateStatus}
              </Badge>
            ) : null}
            {selectedSkill.registryState !== "Present" ? (
              <Badge tone={getRegistryStateTone(selectedSkill.registryState)}>
                {selectedSkill.registryState}
              </Badge>
            ) : null}
            <Badge tone={getWorkspaceSourceTone(selectedSkill.sourceKind)}>
              {selectedSkill.sourceKind}
            </Badge>
            <Badge tone={selectedSkill.enabled ? "success" : "neutral"}>
              {selectedSkill.enabled ? "Enabled" : "Disabled"}
            </Badge>
          </div>

          <div className="grid gap-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
            <DetailField label="Source" value={selectedSkill.sourceRef ?? "Unavailable"} />
            <DetailField label="Registry Path" value={selectedSkill.registryPath ?? "Unavailable"} />
            <DetailField
              label="Version"
              value={selectedSkill.declaredVersion ?? (selectedSkill.registryState === "Deleted" ? "Deleted" : "Unspecified")}
            />
            <DetailField
              label="Registry State"
              value={selectedSkill.registryState}
            />
            <DetailField
              label="Workspace Source"
              value={selectedSkill.sourceKind}
            />
            <DetailField
              label="Resolved From"
              value={selectedSkill.sourceLabel ?? "No workspace binding"}
            />
            {selectedSkill.shadowedLabel ? (
              <DetailField
                label="Shadowed Ancestor"
                value={selectedSkill.shadowedLabel}
              />
            ) : null}
            <DetailField
              label="Last Checked"
              value={selectedSkill.lastCheckedAt ? formatDateTime(selectedSkill.lastCheckedAt, { dateStyle: "medium", timeStyle: "short" }) : pageT("detail.notChecked", "Not checked")}
            />
            <DetailField
              label="Last Updated"
              value={selectedSkill.lastUpdatedAt ? formatDateTime(selectedSkill.lastUpdatedAt, { dateStyle: "medium", timeStyle: "short" }) : pageT("detail.notChecked", "Not checked")}
            />
          </div>

          {selectedSkill.registryState === "Deleted" ? (
            <div className="flex flex-wrap items-start justify-between gap-3 rounded-2xl bg-[var(--color-error-container)] p-4 text-sm text-[var(--color-on-error-container)]">
              <div className="flex min-w-0 items-start gap-2">
                <CircleAlert size={16} aria-hidden="true" className="mt-0.5 shrink-0" />
                <div>
                  <p className="font-medium">The registry copy for this skill is missing.</p>
                  <p className="mt-1 text-[var(--color-on-error-container)]/90">
                    Restore it from Skill Center, then return here and click Refresh to recover this workspace binding.
                  </p>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                <Button
                  size="sm"
                  type="button"
                  variant="secondary"
                  onClick={() => navigate("/settings/skills")}
                >
                  <Package size={14} aria-hidden="true" />
                  Skill Center
                </Button>
              </div>
            </div>
          ) : null}

          {selectedSkill.lastError ? (
            <div className="flex items-start gap-2 rounded-2xl bg-[var(--color-error-container)] p-4 text-sm text-[var(--color-on-error-container)]">
              <CircleAlert size={16} aria-hidden="true" className="mt-0.5 shrink-0" />
              <span>{selectedSkill.lastError}</span>
            </div>
          ) : null}

          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
              Files
            </p>
            <div className="mt-2 flex flex-wrap gap-2">
              {selectedSkill.files.map((file) => (
                <Badge key={file.path} tone={file.isMarkdown ? "accent" : "neutral"}>
                  {file.path}
                </Badge>
              ))}
              {!selectedSkill.files.length ? (
                <span className="text-sm text-[var(--color-on-surface-variant)]">
                  No files discovered.
                </span>
              ) : null}
            </div>
          </div>

          <div>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                SKILL.md Preview
              </p>
              <Button
                size="sm"
                type="button"
                variant="ghost"
                aria-label="Open skill markdown fullscreen"
                onClick={() => setSkillMarkdownFullscreenOpen(true)}
              >
                <Maximize2 size={14} aria-hidden="true" />
                Fullscreen
              </Button>
            </div>
            <div className="mt-2 max-h-[24rem] overflow-y-auto overscroll-y-contain rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-4 py-3">
              <MarkdownContent
                ariaLabel="Skill markdown preview"
                className="max-w-none pr-1"
                content={selectedSkill.skillMarkdown}
              />
            </div>
          </div>
        </>
      )}
    </InspectorPanel>
  );

  useConsoleShellConfig({
    rightPanel,
  });

  return (
    <>
      <PageHeader
        title={pageT("title", "Skills")}
        actions={(
          <>
            <Button
              variant="secondary"
              disabled={refreshing || !activeWorkspaceCode}
              aria-busy={refreshing}
              onClick={() => void handleRefresh()}
            >
              <RefreshCcw size={16} aria-hidden="true" />
              {refreshing ? pageT("actions.refreshing", "Refreshing...") : t("common:actions.refresh")}
            </Button>
            <Button disabled={!activeWorkspaceCode} onClick={() => navigate("/skills/bind")}>
              <Link2 size={16} aria-hidden="true" />
              {pageT("actions.install", "Install")}
            </Button>
          </>
        )}
      />

      <div className="mt-5">
        <SurfaceCard className="p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
            <div className="min-w-[16rem] flex-1">
              <SearchField
                ariaLabel={pageT("search.ariaLabel", "Filter workspace skills")}
                clearLabel={pageT("search.clear", "Clear workspace skill search")}
                placeholder={pageT("search.placeholder", "Filter workspace skills...")}
                value={searchInput}
                onChange={setSearchInput}
              />
            </div>
            {loading ? <Badge tone="accent">{pageT("loading.badge", "Loading")}</Badge> : null}
          </div>
          <DataTable<WorkspaceSkillRecord>
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
                <p>{pageT("emptyState.title", "No workspace skills match the current filters.")}</p>
                <p className="text-xs">
                  {activeWorkspaceCode
                    ? pageT("emptyState.withWorkspace", "Install synced skills to {{workspace}} to see them here.", { workspace: activeWorkspaceLabel })
                    : pageT("emptyState.withoutWorkspace", "Select a workspace in the shell header to view bound skills.")}
                </p>
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
                    {row.sourceRef ? (
                      <span className="block max-w-[22rem] truncate text-[12px] text-[var(--color-on-surface-variant)]">
                        {row.sourceRef}
                      </span>
                    ) : null}
                    <div className="mt-2 flex flex-wrap items-center gap-2">
                      <Badge tone={getRegistryStateTone(row.registryState)}>{row.registryState}</Badge>
                    </div>
                  </div>
                ),
              },
              {
                id: "version",
                header: pageT("columns.version", "Version"),
                render: (row) => row.declaredVersion ?? (row.registryState === "Deleted" ? pageT("status.deleted", "Deleted") : pageT("status.unspecified", "Unspecified")),
              },
              {
                id: "sourceType",
                header: pageT("columns.source", "Source"),
                render: (row) => (
                  row.sourceType ? (
                    <Badge tone={getSourceTone(row.sourceType)}>{row.sourceType}</Badge>
                  ) : (
                    <Badge tone="error">{pageT("status.deleted", "Deleted")}</Badge>
                  )
                ),
              },
              {
                id: "updateStatus",
                header: pageT("columns.update", "Update"),
                render: (row) => (
                  row.updateStatus ? (
                    <Badge tone={getUpdateTone(row.updateStatus)}>{row.updateStatus}</Badge>
                  ) : (
                    <Badge tone="neutral">{pageT("status.unknown", "Unknown")}</Badge>
                  )
                ),
              },
              {
                id: "workspace",
                header: pageT("columns.workspace", "Workspace"),
                render: (row) => (
                  <Badge tone={getWorkspaceSourceTone(row.sourceKind)}>{row.sourceKind}</Badge>
                ),
              },
              {
                id: "enabled",
                header: pageT("columns.enabled", "Enabled"),
                render: (row) => (
                  <BindingSwitch
                    checked={row.enabled}
                    disabled={bindingSkillName === row.skillName}
                    loading={bindingSkillName === row.skillName}
                    onToggle={() => void handleToggleBinding(row)}
                  />
                ),
              },
              {
                id: "action",
                header: pageT("columns.action", "Action"),
                render: (row) => (
                  canUnbindSkill(row.sourceKind) ? (
                    <Button
                      size="sm"
                      type="button"
                      variant="ghost"
                      disabled={bindingSkillName === row.skillName}
                      aria-busy={bindingSkillName === row.skillName}
                      onClick={(event) => {
                        event.stopPropagation();
                        void handleUnbind(row);
                      }}
                    >
                      <Unlink2 size={14} aria-hidden="true" />
                      {pageT("actions.uninstall", "Uninstall")}
                    </Button>
                  ) : (
                    <span className="text-xs text-[var(--color-on-surface-variant)]">
                      {pageT("status.inherited", "Inherited")}
                    </span>
                  )
                ),
              },
            ]}
          />
        </SurfaceCard>
      </div>
      {skillMarkdownFullscreenOpen && selectedSkill ? (
        <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label="Skill markdown fullscreen preview"
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">SKILL.md</h2>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  Preview the rendered markdown for this skill package.
                </p>
              </div>
              <Button
                size="icon"
                type="button"
                variant="ghost"
                aria-label="Close skill markdown fullscreen"
                onClick={() => setSkillMarkdownFullscreenOpen(false)}
              >
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto overscroll-y-contain px-5 py-4">
              <MarkdownContent
                ariaLabel="Skill markdown fullscreen"
                className="max-w-none pr-1"
                content={selectedSkill.skillMarkdown}
              />
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
