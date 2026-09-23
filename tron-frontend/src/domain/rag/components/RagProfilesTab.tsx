import {
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  MoreHorizontal
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { DataTable } from "@/components/data-table";
import { TextInput } from "@/components/forms";
import { TablePagination } from "@/components/table-pagination";
import {
  SurfaceCard,
  useDismissibleLayer,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

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
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";

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
        <DataTable<RetrievalProfileRecord>
          title={pageT("title", "Retrieval profiles")}
          toolbar={(
            <div className="flex flex-wrap items-center gap-3">
              <div className="min-w-[350px]">
                <TextInput
                  placeholder={pageT("searchPlaceholder", "Search retrieval profiles, corpora, routes...")}
                  type="search"
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                />
              </div>
              <Button size={'lg'} onClick={openAddProfile}>
                {pageT("actions.newRetrieval", "New retrieval profile")}
              </Button>
            </div>
          )}
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
                  <div className="flex flex-wrap items-center gap-[12px]">
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
              id: "status",
              header: pageT("table.columns.status", "Status"),
              render: (profile) => <Badge variant={statusTone(profile.status)}>{formatRagStatusLabel(profile.status, t)}</Badge>,
              width: "120px",
            },
            {
              align: "right",
              cellClassName: "w-24",
              header: pageT("table.columns.actions", "Actions"),
              headerClassName: "w-24",
              id: "actions",
              render: (profile) => {
                return (
                  <div className="flex justify-end">
                    <DropdownMenu>
                      <DropdownMenuTrigger>
                        <MoreHorizontal onClick={(e) => e.stopPropagation()} size={16} aria-hidden="true" />
                      </DropdownMenuTrigger>
                      <DropdownMenuContent className="w-40" align="start">
                        <DropdownMenuItem onClick={() => {
                          openProfile(profile);
                        }} disabled={rowBusyId === profile.id}>
                          {pageT("actions.editRetrievalProfile", "Edit retrieval profile")}
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => {
                          onTestProfile(profile);
                        }} disabled={rowBusyId === profile.id}>

                          {pageT("actions.testRetrievalProfile", "Retrieval test")}
                        </DropdownMenuItem>
                        <DropdownMenuItem className='text-destructive-foreground' onClick={() => {
                          void deleteProfile(profile!);
                        }} disabled={rowBusyId === profile?.id}>
                          {pageT("actions.deleteRetrievalProfile", "Delete retrieval profile")}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                );
              },
            },
          ]}
        />
      </SurfaceCard>
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
    <div className="grid content-start gap-[12px]">
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
