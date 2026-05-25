import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  CircleAlert,
  Link2,
  Maximize2,
  Package,
  RefreshCcw,
  Search,
  Unlink2,
  X,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { cn } from "@/shared/lib/utils";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SurfaceCard,
  TextInput,
  TronShell,
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

function SearchField({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="relative">
      <Search
        className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]"
        size={15}
        aria-hidden="true"
      />
      <TextInput
        aria-label="Filter workspace skills"
        className="h-9 w-full pl-9"
        placeholder="Filter workspace skills..."
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

function DetailField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
        {label}
      </p>
      <p className="mt-1 break-words text-sm font-medium text-[var(--color-on-surface)]">
        {value}
      </p>
    </div>
  );
}

function MarkdownContent({
  ariaLabel,
  className,
  content,
}: {
  ariaLabel?: string;
  className?: string;
  content?: string | null;
}) {
  const value = content?.trim();

  if (!value) {
    return (
      <div
        aria-label={ariaLabel}
        className={cn(
          "min-h-6 py-1 text-sm leading-6 text-[var(--color-on-surface)]",
          className,
        )}
      >
        <span className="text-[var(--color-on-surface-variant)]">None</span>
      </div>
    );
  }

  return (
    <div
      aria-label={ariaLabel}
      className={cn(
        "min-h-6 py-1 text-sm leading-7 text-[var(--color-on-surface)]",
        "[&_a]:text-[var(--color-primary)] [&_a]:underline-offset-2 hover:[&_a]:underline",
        "[&_blockquote]:mb-3 [&_blockquote]:border-l-2 [&_blockquote]:border-[var(--color-outline-variant)] [&_blockquote]:pl-4 [&_blockquote]:text-[var(--color-on-surface-variant)]",
        "[&_code]:font-mono [&_code]:text-[13px]",
        "[&_h1]:mb-4 [&_h1]:text-2xl [&_h1]:font-semibold [&_h1]:leading-tight",
        "[&_h2]:mb-3 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:leading-tight",
        "[&_h3]:mb-2 [&_h3]:text-lg [&_h3]:font-semibold [&_h3]:leading-tight",
        "[&_hr]:my-4 [&_hr]:border-[var(--color-outline-variant)]",
        "[&_li]:mb-1",
        "[&_ol]:mb-3 [&_ol]:list-decimal [&_ol]:pl-5",
        "[&_p]:mb-3 [&_p:last-child]:mb-0",
        "[&_p_code]:rounded [&_p_code]:bg-[var(--color-surface-container-lowest)] [&_p_code]:px-1.5 [&_p_code]:py-0.5",
        "[&_pre]:mb-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-[var(--color-surface-container-lowest)] [&_pre]:p-3",
        "[&_pre_code]:bg-transparent [&_pre_code]:p-0",
        "[&_table]:mb-3 [&_table]:w-full [&_table]:border-collapse [&_table]:overflow-hidden [&_table]:rounded-lg",
        "[&_tbody_tr]:border-b [&_tbody_tr]:border-[var(--color-outline-variant)] [&_tbody_tr:last-child]:border-b-0",
        "[&_td]:px-3 [&_td]:py-2 [&_td]:align-top",
        "[&_th]:border-b [&_th]:border-[var(--color-outline-variant)] [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:font-semibold",
        "[&_ul]:mb-3 [&_ul]:list-disc [&_ul]:pl-5",
        className,
      )}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{value}</ReactMarkdown>
    </div>
  );
}

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

function formatTimestamp(value?: string | null) {
  if (!value) return "Not checked";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
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
  const toast = useToast();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode =
    selectedSecondarySpace?.workspaceCode
    ?? selectedPrimarySpace?.workspaceCode
    ?? "";
  const activeWorkspaceLabel =
    selectedSecondarySpace?.label
    ?? selectedPrimarySpace?.name
    ?? "No workspace selected";
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
        error instanceof Error ? error.message : "Workspace skills could not be loaded";
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
          error instanceof Error ? error.message : "Skill detail could not be loaded";
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
      toast.error("Select a workspace in the shell header before refreshing skills");
      return;
    }

    setRefreshing(true);
    try {
      await refreshWorkspaceSkills(activeWorkspaceCode);
      toast.success("Workspace skills refreshed");
      await loadWorkspaceSkills();
      if (selectedSkillName) {
        const detail = await fetchWorkspaceSkill(activeWorkspaceCode, selectedSkillName);
        if (detail.skillName === selectedSkillName) {
          setSelectedSkill(detail);
        }
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Workspace refresh failed";
      toast.error(message);
    } finally {
      setRefreshing(false);
    }
  }

  async function handleToggleBinding(skill: WorkspaceSkillRecord) {
    if (!activeWorkspaceCode) {
      toast.error("Select a workspace in the shell header before enabling skills");
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
            ? `${skill.skillName} reverted to inherited setting in ${activeWorkspaceCode}`
            : `${skill.skillName} removed from ${activeWorkspaceCode}`,
        );
      } else {
        await bindWorkspaceSkill(activeWorkspaceCode, skill.skillName, desiredEnabled);
        toast.success(
          desiredEnabled
            ? `${skill.skillName} enabled in ${activeWorkspaceCode}`
            : `${skill.skillName} disabled in ${activeWorkspaceCode}`,
        );
      }
      await loadWorkspaceSkills();
      if (selectedSkillName === skill.skillName) {
        const detail = await fetchWorkspaceSkill(activeWorkspaceCode, skill.skillName);
        setSelectedSkill(detail);
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Workspace binding could not be updated";
      toast.error(message);
    } finally {
      setBindingSkillName("");
    }
  }

  async function handleUnbind(skill: WorkspaceSkillRecord) {
    if (!activeWorkspaceCode) {
      toast.error("Select a workspace in the shell header before unbinding skills");
      return;
    }
    if (!canUnbindSkill(skill.sourceKind)) {
      toast.info("Inherited skills cannot be unbound from this workspace");
      return;
    }

    setBindingSkillName(skill.skillName);
    try {
      await unbindWorkspaceSkill(activeWorkspaceCode, skill.skillName);
      toast.success(`${skill.skillName} unbound from ${activeWorkspaceCode}`);
      await loadWorkspaceSkills();
      if (selectedSkillName === skill.skillName) {
        const detail = await fetchWorkspaceSkill(activeWorkspaceCode, skill.skillName).catch(() => null);
        setSelectedSkill(detail);
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Workspace binding could not be removed";
      toast.error(message);
    } finally {
      setBindingSkillName("");
    }
  }

  return (
    <TronShell activeItem="Skills" searchPlaceholder="Search installed skills...">
      <PageHeader
        title="Skills"
        actions={(
          <>
            <Button
              variant="secondary"
              disabled={refreshing || !activeWorkspaceCode}
              aria-busy={refreshing}
              onClick={() => void handleRefresh()}
            >
              <RefreshCcw size={16} aria-hidden="true" />
              {refreshing ? "Refreshing..." : "Refresh"}
            </Button>
            <Button disabled={!activeWorkspaceCode} onClick={() => navigate("/skills/bind")}>
              <Link2 size={16} aria-hidden="true" />
              Install
            </Button>
          </>
        )}
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_420px]">
        <SurfaceCard className="p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
            <div className="min-w-[16rem] flex-1">
              <SearchField value={searchInput} onChange={setSearchInput} />
            </div>
            {loading ? <Badge tone="accent">Loading</Badge> : null}
          </div>
          <DataTable<WorkspaceSkillRecord>
            className="rounded-t-none border-0 bg-transparent"
            onRowClick={(row) => setSelectedSkillName(row.skillName)}
            rows={skills}
            rowKey={(row) => row.skillName}
            rowClassName={(row) => (
              row.skillName === selectedSkillName
                ? "bg-[var(--color-surface-container)]"
                : undefined
            )}
            emptyState={(
              <div className="space-y-2">
                <p>No workspace skills match the current filters.</p>
                <p className="text-xs">
                  {activeWorkspaceCode
                    ? `Install synced skills to ${activeWorkspaceLabel} to see them here.`
                    : "Select a workspace in the shell header to view bound skills."}
                </p>
              </div>
            )}
            columns={[
              {
                id: "skillName",
                header: "Skill",
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
                header: "Version",
                render: (row) => row.declaredVersion ?? (row.registryState === "Deleted" ? "Deleted" : "Unspecified"),
              },
              {
                id: "sourceType",
                header: "Source",
                render: (row) => (
                  row.sourceType ? (
                    <Badge tone={getSourceTone(row.sourceType)}>{row.sourceType}</Badge>
                  ) : (
                    <Badge tone="error">Deleted</Badge>
                  )
                ),
              },
              {
                id: "updateStatus",
                header: "Update",
                render: (row) => (
                  row.updateStatus ? (
                    <Badge tone={getUpdateTone(row.updateStatus)}>{row.updateStatus}</Badge>
                  ) : (
                    <Badge tone="neutral">Unknown</Badge>
                  )
                ),
              },
              {
                id: "workspace",
                header: "Workspace",
                render: (row) => (
                  <Badge tone={getWorkspaceSourceTone(row.sourceKind)}>{row.sourceKind}</Badge>
                ),
              },
              {
                id: "enabled",
                header: "Enabled",
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
                header: "Action",
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
                      Uninstall
                    </Button>
                  ) : (
                    <span className="text-xs text-[var(--color-on-surface-variant)]">
                      Inherited
                    </span>
                  )
                ),
              },
            ]}
          />
        </SurfaceCard>

        <SurfaceCard className="space-y-5">
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
              Skill Detail
            </p>
            <h2 className="mt-2 text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">
              {(selectedSkill?.skillName ?? selectedSkillName) || "Select a Skill"}
            </h2>
          </div>

          {detailLoading ? (
            <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
              Loading skill detail...
            </div>
          ) : !selectedSkill ? (
            <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
              No bound skill is selected.
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
                  value={formatTimestamp(selectedSkill.lastCheckedAt)}
                />
                <DetailField
                  label="Last Updated"
                  value={formatTimestamp(selectedSkill.lastUpdatedAt)}
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
    </TronShell>
  );
}
