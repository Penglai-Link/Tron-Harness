import {
  type SyntheticEvent,
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  Archive,
  ExternalLink,
  MoreHorizontal,
  Pencil,
  RotateCcw,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  DataTable,
  PageHeader,
  SelectInput,
  TablePagination,
  TextInput,
  useToast,
  useLocaleFormatters,
  useWorkspaceSpaces,
} from "@/shared";

import {
  archiveApp,
  restoreApp,
} from "@/domain/app/api";
import {
  AppRuntimeRoleBadge,
  AppStatusBadge,
  AppTagList,
} from "@/domain/app/components";
import { useAppList } from "@/domain/app/hooks";
import {
  archiveMockApp,
  restoreMockApp,
} from "@/domain/app/mock";
import type {
  AppListItem,
  AppRuntimeRole,
} from "@/domain/app/model";
import { formatAppTimestamp } from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { SelectItem } from "@/components/ui/select";

type StatusFilter = "Active" | "Archived" | "All";

function stopRowNavigation(event: SyntheticEvent<HTMLElement>) {
  event.stopPropagation();
}

function openExternalLink(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

function AppActions({
  app,
  onArchive,
  onRestore,
  pendingLifecycle,
}: {
  app: AppListItem;
  onArchive: () => void;
  onRestore: () => void;
  pendingLifecycle: boolean;
}) {
  const navigate = useNavigate();
  const { t } = useTranslation("pages");
  const pageT = (key: string, options?: Record<string, unknown>) => t(`apps.${key}`, options);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger>
        <Button
          aria-label={pageT("aria.actionsFor", { name: app.name })}
          size="icon"
          type="button"
          variant="ghost"
          onClick={(event) => event.stopPropagation()}
        >
          <MoreHorizontal size={16} aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" onClick={stopRowNavigation}>
        <DropdownMenuItem
          onClick={(event) => {
            event.stopPropagation();
            navigate(`/apps/edit?appCode=${encodeURIComponent(app.appCode)}`);
          }}
        >
          {pageT("actions.edit")}
        </DropdownMenuItem>
        {app.status === "Archived" ? (
          <DropdownMenuItem
            disabled={pendingLifecycle}
            onClick={(event) => {
              event.stopPropagation();
              onRestore();
            }}
          >
            {pageT("actions.restore")}
          </DropdownMenuItem>
        ) : (
          <DropdownMenuItem
            disabled={pendingLifecycle}
            onClick={(event) => {
              event.stopPropagation();
              onArchive();
            }}
          >
            {pageT("actions.archive")}
          </DropdownMenuItem>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem
          disabled={!app.repositoryUrl}
          onClick={(event) => {
            event.stopPropagation();
            if (app.repositoryUrl) {
              openExternalLink(app.repositoryUrl);
            }
          }}
        >
          {pageT("actions.gitlab")}
        </DropdownMenuItem>
        <DropdownMenuItem disabled>
          {pageT("actions.poseidon")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function AppsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation("pages");
  const pageT = (key: string, options?: Record<string, unknown>) => t(`apps.${key}`, options);
  const toast = useToast();
  const { currentLocale } = useLocaleFormatters();
  const dataSource = getServiceDataSource("app");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [runtimeRole, setRuntimeRole] = useState<"" | AppRuntimeRole>("");
  const [status, setStatus] = useState<StatusFilter>("Active");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [pendingLifecycleCode, setPendingLifecycleCode] = useState<string | null>(null);
  const query = useMemo(() => ({
    current,
    includeArchived: status === "All",
    runtimeRole: runtimeRole || undefined,
    search: debouncedSearch || undefined,
    size,
    status: status === "All" ? undefined : status,
    workspaceCode: activeWorkspaceCode,
  }), [activeWorkspaceCode, current, debouncedSearch, runtimeRole, size, status]);
  const { loading, page, refresh } = useAppList(query);
  const pageCount = Math.max(1, Math.ceil(page.total / page.size));

  useEffect(() => {
    setCurrent(1);
  }, [activeWorkspaceCode, runtimeRole, searchInput, status]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(searchInput.trim());
    }, 280);

    return () => window.clearTimeout(timer);
  }, [searchInput]);

  useEffect(() => {
    if (current > pageCount) {
      setCurrent(pageCount);
    }
  }, [current, pageCount]);

  async function updateLifecycle(app: AppListItem, action: "archive" | "restore") {
    setPendingLifecycleCode(app.appCode);
    try {
      if (action === "archive") {
        if (dataSource === "api") {
          await archiveApp(app.appCode);
        } else {
          archiveMockApp(app.appCode);
        }
        toast.success(pageT("messages.archived", { name: app.name }));
      } else {
        if (dataSource === "api") {
          await restoreApp(app.appCode);
        } else {
          restoreMockApp(app.appCode);
        }
        toast.success(pageT("messages.restored", { name: app.name }));
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("messages.lifecycleUpdateFailed"));
    } finally {
      setPendingLifecycleCode(null);
    }
  }

  return (
    <>
      <PageHeader
        title={pageT("title")}
        actions={(
          <>
            <Button
              type="button"
              size="lg"
              variant="outline"
              disabled={loading}
              onClick={() => {
                refresh();
                toast.success(pageT("messages.refreshed"));
              }}
            >
              {pageT("actions.refresh")}
            </Button>
            <Button size="lg" type="button" disabled={!activeWorkspaceCode} onClick={() => navigate("/apps/new")}>
              {pageT("actions.new")}
            </Button>
          </>
        )}
      />
      <DataTable<AppListItem>
        rows={page.records}
        rowKey={(row) => row.appCode}
        onRowClick={(app) => navigate(`/apps/detail?appCode=${encodeURIComponent(app.appCode)}`)}
        emptyState={activeWorkspaceCode
          ? pageT("empty.noMatch")
          : pageT("empty.workspace")}
        toolbar={(
          <div className="w-full grid gap-4 lg:grid-cols-3 sm:grid-cols-1">
            <TextInput
              aria-label={pageT("aria.search")}
              placeholder={pageT("placeholders.search")}
              type="search"
              value={searchInput}
              onChange={(event) => setSearchInput(event.target.value)}
            />
            <SelectInput aria-label={pageT("aria.runtimeRole")} value={runtimeRole} onValueChange={(event) => setRuntimeRole(event as "" | AppRuntimeRole)}>
              <SelectItem value="">{pageT("filters.allRoles")}</SelectItem>
              <SelectItem value="Frontend">{pageT("runtimeRole.Frontend")}</SelectItem>
              <SelectItem value="Backend">{pageT("runtimeRole.Backend")}</SelectItem>
              <SelectItem value="Mono">{pageT("runtimeRole.Mono")}</SelectItem>
            </SelectInput>
            <SelectInput aria-label={pageT("aria.status")} value={status} onValueChange={(event) => setStatus(event as StatusFilter)}>
              <SelectItem value="Active">{pageT("filters.active")}</SelectItem>
              <SelectItem value="Archived">{pageT("filters.archived")}</SelectItem>
              <SelectItem value="All">{pageT("filters.allStatuses")}</SelectItem>
            </SelectInput>
          </div>
        )}
        pagination={(
          <TablePagination
            itemLabel={pageT("pagination.itemLabel")}
            endItem={Math.min(page.total, (page.current - 1) * page.size + page.records.length)}
            page={page.current}
            pageCount={pageCount}
            pageSize={page.size}
            startItem={page.total ? ((page.current - 1) * page.size) + 1 : 0}
            totalItems={page.total}
            onPageChange={setCurrent}
            onPageSizeChange={(value) => {
              setSize(value);
              setCurrent(1);
            }}
          />
        )}
        columns={[
          {
            id: "name",
            header: pageT("columns.name"),
            render: (app) => (
              <div>
                <p className="text-sm font-semibold text-[var(--color-on-surface)]">{app.name}</p>
                <p className="mt-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">{app.appCode}</p>
              </div>
            ),
          },
          {
            id: "description",
            header: pageT("columns.description"),
            render: (app) => (
              <p className="max-w-72 text-sm leading-5 text-[var(--color-on-surface-variant)]">
                {app.description ?? pageT("detail.notSet")}
              </p>
            ),
          },
          {
            id: "runtimeRole",
            header: pageT("columns.runtime"),
            render: (app) => <AppRuntimeRoleBadge runtimeRole={app.runtimeRole} />,
          },
          {
            id: "repository",
            header: pageT("columns.repository"),
            render: (app) => (
              <div className="space-y-1">
                {app.repositoryUrl ? (
                  <a
                    className="inline-flex max-w-[13rem] items-start gap-1 text-sm text-[var(--color-primary)] underline decoration-[0.08em] underline-offset-2 transition-opacity hover:opacity-80"
                    href={app.repositoryUrl}
                    rel="noreferrer"
                    target="_blank"
                    onClick={stopRowNavigation}
                    onMouseDown={stopRowNavigation}
                    onKeyDown={stopRowNavigation}
                    title={app.repositoryUrl}
                  >
                    <span
                      className="min-w-0 break-all leading-5"
                      style={{
                        WebkitBoxOrient: "vertical",
                        WebkitLineClamp: 2,
                        display: "-webkit-box",
                        overflow: "hidden",
                      }}
                    >
                      {app.repositoryUrl}
                    </span>
                    <ExternalLink className="mt-0.5 shrink-0" size={12} aria-hidden="true" />
                  </a>
                ) : (
                  <p className="break-all text-sm text-[var(--color-on-surface)]">{pageT("detail.notSet")}</p>
                )}
              </div>
            ),
          },
          {
            id: "tags",
            header: pageT("columns.tags"),
            render: (app) => <AppTagList tags={app.tags} />,
          },
          {
            id: "status",
            header: pageT("columns.status"),
            render: (app) => <AppStatusBadge status={app.status} />,
          },
          {
            id: "updatedAt",
            header: pageT("columns.updated"),
            render: (app) => <span>{formatAppTimestamp(app.updatedAt, currentLocale)}</span>,
          },
          {
            id: "actions",
            align: "right",
            header: pageT("columns.actions"),
            render: (app) => (
              <AppActions
                app={app}
                onArchive={() => void updateLifecycle(app, "archive")}
                onRestore={() => void updateLifecycle(app, "restore")}
                pendingLifecycle={pendingLifecycleCode === app.appCode}
              />
            ),
          },
        ]}
      />
    </>
  );
}
