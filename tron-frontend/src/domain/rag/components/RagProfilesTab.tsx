import {
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  MoreHorizontal,
  Search,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  DataTable,
  SurfaceCard,
  TablePagination,
  TextInput,
  useDismissibleLayer,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";

import type {
  ProfileRecord,
  RagRetrievalRouteConfig,
  RetrievalProfileRecord,
} from "@/domain/rag/model/RagModel";
import {
  formatRagStatusLabel,
  Pill,
  SectionTitle,
  statusTone,
} from "@/domain/rag/components/RagShared";

interface RowMenuState {
  left: number;
  profileId: string;
  top: number;
  triggerElement: HTMLButtonElement;
}

function activeRetrievalRoutes(profile: RetrievalProfileRecord) {
  return profile.config.routesConfig.filter((route) => (
    route.enabled !== false
    && route.corpusId.trim()
    && route.strategyId.trim()
  ));
}

function uniqueRouteCorpusIds(profile: RetrievalProfileRecord) {
  return Array.from(new Set(activeRetrievalRoutes(profile).map((route) => route.corpusId.trim()).filter(Boolean)));
}

function routeTypeLabel(route: RagRetrievalRouteConfig) {
  const routeType = String(route.routeType || "").trim();
  return routeType && routeType !== "ROUTE_SEARCH" ? routeType : route.strategyId.trim();
}

function routeTypeCounts(routes: RagRetrievalRouteConfig[]) {
  const counts = new Map<string, number>();
  routes.forEach((route) => {
    const label = routeTypeLabel(route);
    if (!label) return;
    counts.set(label, (counts.get(label) ?? 0) + 1);
  });
  return Array.from(counts.entries());
}

function normalizeSearchValue(value: string) {
  return value.trim().toLowerCase();
}

function profileMatchesQuery(profile: RetrievalProfileRecord, query: string) {
  const normalizedQuery = normalizeSearchValue(query);
  if (!normalizedQuery) return true;
  const activeRoutes = activeRetrievalRoutes(profile);
  const searchableValues = [
    profile.id,
    profile.description,
    profile.status,
    profile.primaryCorpusId,
    ...uniqueRouteCorpusIds(profile),
    ...activeRoutes.map((route) => route.strategyId),
    ...activeRoutes.map((route) => routeTypeLabel(route)),
  ];
  return searchableValues.some((value) => value.toLowerCase().includes(normalizedQuery));
}

export function RagProfilesTab({
  onDeleteProfile,
  onTestProfile,
  profiles,
}: {
  onDeleteProfile: (profile: RetrievalProfileRecord) => Promise<void> | void;
  onTestProfile: (profile: RetrievalProfileRecord) => void;
  profiles: ProfileRecord[];
}) {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages"]);
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [query, setQuery] = useState("");
  const [rowBusyId, setRowBusyId] = useState("");
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.profiles.${key}`, { defaultValue, ...options })
  );
  const retrievalProfiles = profiles.filter((profile): profile is RetrievalProfileRecord => profile.type === "Retrieval");
  const filteredRetrievalProfiles = retrievalProfiles.filter((profile) => profileMatchesQuery(profile, query));
  const retrievalPagination = useTablePagination(filteredRetrievalProfiles, 10);
  const openMenuProfile = openRowMenu
    ? retrievalProfiles.find((profile) => profile.id === openRowMenu.profileId) ?? null
    : null;

  useDismissibleLayer({
    open: Boolean(openRowMenu),
    onDismiss: () => setOpenRowMenu(null),
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement ?? null,
  });

  function openAddProfile() {
    navigate("/rag/profiles/retrieval/new?returnProfileType=retrieval");
  }

  function openProfile(profile: RetrievalProfileRecord) {
    navigate(`/rag/profiles/retrieval/${encodeURIComponent(profile.id)}/edit?returnProfileType=retrieval`);
  }

  function toggleRowMenu(profile: RetrievalProfileRecord, triggerElement: HTMLButtonElement) {
    const rect = triggerElement.getBoundingClientRect();
    const menuWidth = 176;
    const menuHeight = 136;
    const preferredTop = rect.bottom + 8;
    const top = preferredTop + menuHeight > window.innerHeight - 16
      ? Math.max(16, rect.top - menuHeight - 8)
      : preferredTop;
    setOpenRowMenu((current) => (
      current?.profileId === profile.id
        ? null
        : {
          left: Math.max(16, Math.min(rect.right - menuWidth, window.innerWidth - menuWidth - 16)),
          profileId: profile.id,
          top,
          triggerElement,
        }
    ));
  }

  async function deleteProfile(profile: RetrievalProfileRecord) {
    if (rowBusyId === profile.id) return;
    setRowBusyId(profile.id);
    setOpenRowMenu(null);
    try {
      await onDeleteProfile(profile);
    } finally {
      setRowBusyId("");
    }
  }

  return (
    <>
      <SurfaceCard className="p-0">
        <div className="border-b border-[var(--color-outline-variant)] px-5 py-4">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <SectionTitle
              icon={<Search size={17} aria-hidden="true" />}
              title={pageT("title", "Retrieval profiles")}
            />
          </div>
          <div className="mt-4 grid gap-3 md:grid-cols-[minmax(260px,1fr)_auto] md:items-center">
            <div className="relative min-w-0">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
              <TextInput
                className="w-full pl-9"
                placeholder={pageT("searchPlaceholder", "Search retrieval profiles, corpora, routes, or strategies...")}
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </div>
            <Button className="justify-self-start md:justify-self-end" onClick={openAddProfile}>
              {pageT("actions.newRetrieval", "New retrieval profile")}
            </Button>
          </div>
        </div>

        <DataTable<RetrievalProfileRecord>
          rows={retrievalPagination.visibleItems}
          rowKey={(profile) => profile.id}
          variant="embedded"
          onRowClick={openProfile}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "retrieval profiles")} {...retrievalPagination.paginationProps} />}
          emptyState={pageT("empty.retrieval", "No retrieval profiles.")}
          columns={[
            {
              id: "profile",
              header: pageT("table.columns.profile", "Profile"),
              render: (profile) => (
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      aria-label={pageT("row.editAria", "Edit {{id}}", { id: profile.id })}
                      className="truncate text-left font-medium hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-elevated-surface)]"
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        openProfile(profile);
                      }}
                    >
                      {profile.id}
                    </button>
                    <Badge tone={statusTone(profile.status)}>{formatRagStatusLabel(profile.status, t)}</Badge>
                  </div>
                  {profile.description ? (
                    <p className="mt-1 line-clamp-2 text-[13px] text-[var(--color-on-surface-variant)]">{profile.description}</p>
                  ) : null}
                </div>
              ),
            },
            {
              id: "corpus",
              header: pageT("table.columns.corpus", "Corpus"),
              render: (profile) => {
                const routeCorpusIds = uniqueRouteCorpusIds(profile);
                return routeCorpusIds.length > 0 ? (
                  <div className="flex flex-wrap gap-1.5">
                    {routeCorpusIds.map((corpusId) => <Pill key={corpusId}>{corpusId}</Pill>)}
                  </div>
                ) : (
                  <span className="text-[13px] text-[var(--color-on-surface-variant)]">
                    {pageT("row.noRouteCorpora", "No route corpora")}
                  </span>
                );
              },
            },
            {
              id: "routes",
              header: pageT("table.columns.routes", "Routes"),
              render: (profile) => <RetrievalProfileSummary profile={profile} />,
            },
            {
              align: "right",
              cellClassName: "w-24",
              header: pageT("table.columns.actions", "Actions"),
              headerClassName: "w-24",
              id: "actions",
              render: (profile) => {
                const isMenuOpen = openRowMenu?.profileId === profile.id;
                return (
                  <div className="flex justify-end">
                    <Button
                      aria-expanded={isMenuOpen}
                      aria-label={pageT("actions.openRowActions", "Open actions for {{id}}", { id: profile.id })}
                      disabled={rowBusyId === profile.id}
                      size="icon"
                      type="button"
                      variant="ghost"
                      onClick={(event) => {
                        event.stopPropagation();
                        toggleRowMenu(profile, event.currentTarget);
                      }}
                    >
                      <MoreHorizontal size={16} aria-hidden="true" />
                    </Button>
                  </div>
                );
              },
            },
          ]}
        />
      </SurfaceCard>

      {openRowMenu && openMenuProfile
        ? createPortal(
          <div
            ref={rowMenuRef}
            aria-label={pageT("actions.rowMenuAria", "Actions for {{id}}", { id: openMenuProfile.id })}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
          >
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyId === openMenuProfile.id}
              role="menuitem"
              type="button"
              onClick={() => {
                setOpenRowMenu(null);
                openProfile(openMenuProfile);
              }}
            >
              {pageT("actions.editRetrievalProfile", "Edit retrieval profile")}
            </button>
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyId === openMenuProfile.id}
              role="menuitem"
              type="button"
              onClick={() => {
                setOpenRowMenu(null);
                onTestProfile(openMenuProfile);
              }}
            >
              {pageT("actions.testRetrievalProfile", "Retrieval test")}
            </button>
            <button
              className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyId === openMenuProfile.id}
              role="menuitem"
              type="button"
              onClick={() => {
                void deleteProfile(openMenuProfile);
              }}
            >
              {pageT("actions.deleteRetrievalProfile", "Delete retrieval profile")}
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}

function RetrievalProfileSummary({ profile }: { profile: RetrievalProfileRecord }) {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.profiles.${key}`, { defaultValue, ...options })
  );
  const enabledRoutes = activeRetrievalRoutes(profile);
  const routeTypes = routeTypeCounts(enabledRoutes);
  return (
    <div className="grid content-start gap-2">
      <span className="text-[12px] text-[var(--color-on-surface-variant)]">
        {pageT("row.enabledRoutes", "{{count}} routes", { count: enabledRoutes.length })}
      </span>
      {routeTypes.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {routeTypes.map(([routeType, count]) => (
            <Pill key={routeType}>{count > 1 ? `${routeType} x${count}` : routeType}</Pill>
          ))}
        </div>
      ) : null}
    </div>
  );
}
