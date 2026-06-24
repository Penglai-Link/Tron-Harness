import {
  type CSSProperties,
  type MouseEvent,
  type SyntheticEvent,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  Archive,
  ExternalLink,
  MoreHorizontal,
  Pencil,
  Plus,
  RefreshCcw,
  RotateCcw,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SelectInput,
  TablePagination,
  TextInput,
  useDismissibleLayer,
  useToast,
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

type StatusFilter = "Active" | "Archived" | "All";
type OpenRowMenuState = {
  anchorRect: DOMRect;
  appCode: string;
  triggerElement: HTMLButtonElement;
};

const ACTION_MENU_GAP = 8;
const ACTION_MENU_HEIGHT = 196;
const ACTION_MENU_MARGIN = 12;
const ACTION_MENU_WIDTH = 176;

function stopRowNavigation(event: SyntheticEvent<HTMLElement>) {
  event.stopPropagation();
}

function getActionMenuStyle(anchorRect: DOMRect): CSSProperties {
  const left = Math.min(
    window.innerWidth - ACTION_MENU_WIDTH - ACTION_MENU_MARGIN,
    Math.max(ACTION_MENU_MARGIN, anchorRect.right - ACTION_MENU_WIDTH),
  );
  const top = anchorRect.bottom + ACTION_MENU_GAP + ACTION_MENU_HEIGHT <= window.innerHeight - ACTION_MENU_MARGIN
    ? anchorRect.bottom + ACTION_MENU_GAP
    : Math.max(ACTION_MENU_MARGIN, anchorRect.top - ACTION_MENU_GAP - ACTION_MENU_HEIGHT);

  return {
    left,
    position: "fixed",
    top,
    width: ACTION_MENU_WIDTH,
  };
}

function openExternalLink(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

function AppActions({
  app,
  isOpen,
  onToggleOpen,
}: {
  app: AppListItem;
  isOpen: boolean;
  onToggleOpen: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return (
    <div className="flex justify-end">
      <Button
        aria-expanded={isOpen}
        aria-haspopup="menu"
        aria-label={`Open actions for ${app.name}`}
        size="icon"
        type="button"
        variant="ghost"
        onClick={(event) => {
          event.stopPropagation();
          onToggleOpen(event);
        }}
      >
        <MoreHorizontal size={16} aria-hidden="true" />
      </Button>
    </div>
  );
}

export function AppsPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("app");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [runtimeRole, setRuntimeRole] = useState<"" | AppRuntimeRole>("");
  const [status, setStatus] = useState<StatusFilter>("Active");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [openRowMenu, setOpenRowMenu] = useState<OpenRowMenuState | null>(null);
  const [pendingLifecycleCode, setPendingLifecycleCode] = useState<string | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
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
    setOpenRowMenu(null);
  }, [activeWorkspaceCode, current, runtimeRole, searchInput, size, status]);

  useEffect(() => {
    if (current > pageCount) {
      setCurrent(pageCount);
    }
  }, [current, pageCount]);

  useDismissibleLayer({
    onDismiss: () => setOpenRowMenu(null),
    open: openRowMenu !== null,
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement,
  });

  const openMenuApp = useMemo(
    () => (openRowMenu ? page.records.find((app) => app.appCode === openRowMenu.appCode) ?? null : null),
    [openRowMenu, page.records],
  );

  async function updateLifecycle(app: AppListItem, action: "archive" | "restore") {
    setOpenRowMenu(null);
    setPendingLifecycleCode(app.appCode);
    try {
      if (action === "archive") {
        if (dataSource === "api") {
          await archiveApp(app.appCode);
        } else {
          archiveMockApp(app.appCode);
        }
        toast.success(`${app.name} archived`);
      } else {
        if (dataSource === "api") {
          await restoreApp(app.appCode);
        } else {
          restoreMockApp(app.appCode);
        }
        toast.success(`${app.name} restored`);
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : `App lifecycle update failed for ${app.appCode}`);
    } finally {
      setPendingLifecycleCode(null);
    }
  }

  return (
    <>
      <PageHeader
        title="App Management"
        actions={(
          <>
            <Button
              type="button"
              variant="secondary"
              disabled={loading}
              onClick={() => {
                setOpenRowMenu(null);
                refresh();
                toast.success("App registry refreshed");
              }}
            >
              <RefreshCcw size={15} aria-hidden="true" />
              Refresh
            </Button>
            <Button type="button" disabled={!activeWorkspaceCode} onClick={() => navigate("/apps/new")}>
              <Plus size={15} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />
      <DataTable<AppListItem>
        rows={page.records}
        rowKey={(row) => row.appCode}
        onRowClick={(app) => navigate(`/apps/detail?appCode=${encodeURIComponent(app.appCode)}`)}
        emptyState={activeWorkspaceCode
          ? "No apps match the current filters."
          : "Select a workspace in the shell header to load apps."}
        toolbar={(
          <>
            <TextInput
              aria-label="Search apps"
              className="h-8 min-w-[20rem] w-[30rem] max-w-full"
              placeholder="Search apps, repos, branches, or tags..."
              type="search"
              value={searchInput}
              onChange={(event) => setSearchInput(event.target.value)}
            />
            <SelectInput aria-label="Runtime role filter" className="h-8 min-w-36" value={runtimeRole} onChange={(event) => setRuntimeRole(event.target.value as "" | AppRuntimeRole)}>
              <option value="">All roles</option>
              <option value="Frontend">Frontend</option>
              <option value="Backend">Backend</option>
              <option value="Mono">Mono</option>
            </SelectInput>
            <SelectInput aria-label="App status filter" className="h-8 min-w-36" value={status} onChange={(event) => setStatus(event.target.value as StatusFilter)}>
              <option value="Active">Active</option>
              <option value="Archived">Archived</option>
              <option value="All">All</option>
            </SelectInput>
          </>
        )}
        pagination={(
          <TablePagination
            itemLabel="apps"
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
            header: "Name",
            render: (app) => (
              <div>
                <p className="text-sm font-semibold text-[var(--color-on-surface)]">{app.name}</p>
                <p className="mt-1 font-mono text-[11px] text-[var(--color-on-surface-variant)]">{app.appCode}</p>
              </div>
            ),
          },
          {
            id: "description",
            header: "Description",
            render: (app) => (
              <p className="max-w-72 text-sm leading-5 text-[var(--color-on-surface-variant)]">
                {app.description ?? "-"}
              </p>
            ),
          },
          {
            id: "runtimeRole",
            header: "Runtime",
            render: (app) => <AppRuntimeRoleBadge runtimeRole={app.runtimeRole} />,
          },
          {
            id: "repository",
            header: "Repository",
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
                  <p className="break-all text-sm text-[var(--color-on-surface)]">Not set</p>
                )}
              </div>
            ),
          },
          {
            id: "tags",
            header: "Tags",
            render: (app) => <AppTagList tags={app.tags} />,
          },
          {
            id: "status",
            header: "Status",
            render: (app) => <AppStatusBadge status={app.status} />,
          },
          {
            id: "updatedAt",
            header: "Updated",
            render: (app) => <span>{formatAppTimestamp(app.updatedAt)}</span>,
          },
          {
            id: "actions",
            align: "right",
            header: "Actions",
            render: (app) => (
                <AppActions
                  app={app}
                  isOpen={openRowMenu?.appCode === app.appCode}
                  onToggleOpen={(event) => {
                    const anchorRect = event.currentTarget.getBoundingClientRect();
                    setOpenRowMenu((currentMenu) => (
                      currentMenu?.appCode === app.appCode
                        ? null
                        : {
                          appCode: app.appCode,
                          anchorRect,
                          triggerElement: event.currentTarget,
                        }
                    ));
                  }}
                />
              ),
            },
          ]}
      />
      {openMenuApp && openRowMenu
        ? createPortal(
          <div
            className="z-50 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface-strong)] p-2 shadow-[0_12px_28px_rgba(0,0,0,0.18)]"
            data-app-actions-menu
            ref={rowMenuRef}
            role="menu"
            aria-label={`Actions for ${openMenuApp.name}`}
            style={getActionMenuStyle(openRowMenu.anchorRect)}
            onClick={stopRowNavigation}
          >
            <button
              className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              role="menuitem"
              type="button"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/apps/edit?appCode=${encodeURIComponent(openMenuApp.appCode)}`);
              }}
            >
              <Pencil size={14} aria-hidden="true" />
              Edit
            </button>
            {openMenuApp.status === "Archived" ? (
              <button
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                role="menuitem"
                type="button"
                disabled={pendingLifecycleCode === openMenuApp.appCode}
                onClick={() => void updateLifecycle(openMenuApp, "restore")}
              >
                <RotateCcw size={14} aria-hidden="true" />
                Restore
              </button>
            ) : (
              <button
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                role="menuitem"
                type="button"
                disabled={pendingLifecycleCode === openMenuApp.appCode}
                onClick={() => void updateLifecycle(openMenuApp, "archive")}
            >
              <Archive size={14} aria-hidden="true" />
              Archive
            </button>
            )}
            <div className="my-1 border-t border-[var(--color-outline-variant)]" aria-hidden="true" />
            <button
              className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              role="menuitem"
              type="button"
              disabled={!openMenuApp.repositoryUrl}
              onClick={() => {
                if (!openMenuApp.repositoryUrl) {
                  return;
                }
                setOpenRowMenu(null);
                openExternalLink(openMenuApp.repositoryUrl);
              }}
            >
              <ExternalLink size={14} aria-hidden="true" />
              Gitlab
            </button>
            <button
              className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-[var(--color-on-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              role="menuitem"
              type="button"
              disabled
            >
              <ExternalLink size={14} aria-hidden="true" />
              Poseidon
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}
