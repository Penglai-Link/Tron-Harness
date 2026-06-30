import {
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  MoreHorizontal,
  Search,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import {
  DataTable,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  useDismissibleLayer,
  useTablePagination,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";

import type { CorpusRecord } from "@/domain/rag/model/RagModel";
import {
  formatRagStatusLabel,
  statusTone,
} from "@/domain/rag/components/RagShared";
import { SelectItem } from "@/components/ui/select";

interface RowMenuState {
  code: string;
  left: number;
  top: number;
  triggerElement: HTMLButtonElement;
}

export function RagCorporaTab({
  corpusPagination,
  query,
  statusFilter,
  onCreateCorpus,
  onDeleteCorpus,
  onDeleteIngestProfile,
  onEditCorpus,
  onTestIngest,
  onQueryChange,
  onStatusFilterChange,
}: {
  corpusPagination: ReturnType<typeof useTablePagination<CorpusRecord>>;
  query: string;
  statusFilter: string;
  onCreateCorpus: () => void;
  onDeleteCorpus: (corpus: CorpusRecord) => Promise<void> | void;
  onDeleteIngestProfile: (corpus: CorpusRecord) => Promise<void> | void;
  onEditCorpus: (corpus: CorpusRecord) => void;
  onTestIngest: (corpus: CorpusRecord) => void;
  onQueryChange: (value: string) => void;
  onStatusFilterChange: (value: string) => void;
}) {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages"]);
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
  const openMenuCorpus = openRowMenu
    ? corpusPagination.visibleItems.find((corpus) => corpus.code === openRowMenu.code) ?? null
    : null;
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:rag.corpora.${key}`, { defaultValue, ...options })
  );

  useDismissibleLayer({
    open: Boolean(openRowMenu),
    onDismiss: () => setOpenRowMenu(null),
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement ?? null,
  });

  function toggleRowMenu(corpus: CorpusRecord, triggerElement: HTMLButtonElement) {
    const rect = triggerElement.getBoundingClientRect();
    const menuWidth = 176;
    const menuHeight = corpus.ingestProfile.trim() ? 188 : 108;
    const preferredTop = rect.bottom + 8;
    const top = preferredTop + menuHeight > window.innerHeight - 16
      ? Math.max(16, rect.top - menuHeight - 8)
      : preferredTop;
    setOpenRowMenu((current) => (
      current?.code === corpus.code
        ? null
        : {
          code: corpus.code,
          left: Math.max(16, Math.min(rect.right - menuWidth, window.innerWidth - menuWidth - 16)),
          top,
          triggerElement,
        }
    ));
  }

  async function runRowAction(corpus: CorpusRecord, action: (target: CorpusRecord) => Promise<void> | void) {
    if (rowBusyCode === corpus.code) return;
    setRowBusyCode(corpus.code);
    setOpenRowMenu(null);
    try {
      await action(corpus);
    } finally {
      setRowBusyCode("");
    }
  }

  function ingestProfilePath(corpus: CorpusRecord) {
    const ingestProfileId = corpus.ingestProfile.trim();
    if (ingestProfileId) {
      return `/rag/profiles/ingest/${encodeURIComponent(ingestProfileId)}/edit?returnProfileType=ingest`;
    }
    return `/rag/profiles/ingest/new?returnProfileType=ingest&corpusId=${encodeURIComponent(corpus.code)}`;
  }

  function openIngestProfile(corpus: CorpusRecord) {
    setOpenRowMenu(null);
    navigate(ingestProfilePath(corpus));
  }

  return (
    <>
      <SurfaceCard className="p-0">
        <div className="border-b border-[var(--color-outline-variant)] px-5 py-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h2 className="text-base font-semibold">{pageT("table.title", "Corpus list")}</h2>
          </div>
          <div className="mt-4 grid gap-3 md:grid-cols-[minmax(260px,1fr)_180px_auto] md:items-center">
            <div className="relative min-w-0">
              <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
              <TextInput
                className="w-full pl-9"
                placeholder={pageT("searchPlaceholder", "Search corpora, codes, or descriptions...")}
                type="search"
                value={query}
                onChange={(event) => onQueryChange(event.target.value)}
              />
            </div>
            <SelectInput
              className="w-full"
              value={statusFilter}
              onValueChange={(event) => onStatusFilterChange(event!)}
            >
              {["All", "Active", "Indexing", "Warning", "Disabled"].map((status) => (
                <SelectItem key={status} value={status}>
                  {status === "All" ? pageT("filters.allStatuses", "All statuses") : formatRagStatusLabel(status, t)}
                </SelectItem>
              ))}
            </SelectInput>
            <Button className="justify-self-start md:justify-self-end" size="sm" variant="secondary" onClick={onCreateCorpus}>
              <Sparkles size={17} aria-hidden="true" />
              {pageT("actions.newCorpus", "New corpus")}
            </Button>
          </div>
        </div>

        <DataTable
          rows={corpusPagination.visibleItems}
          rowKey={(corpus) => corpus.code}
          variant="embedded"
          onRowClick={onEditCorpus}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "corpora")} {...corpusPagination.paginationProps} />}
          columns={[
            {
              id: "corpus",
              header: pageT("table.columns.corpus", "Corpus"),
              render: (corpus) => (
                <div>
                  <p className="font-medium">{corpus.name}</p>
                  <p className="text-[13px] text-[var(--color-on-surface-variant)]">{corpus.code}</p>
                </div>
              ),
            },
            {
              id: "description",
              header: pageT("table.columns.description", "Description"),
              render: (corpus) => (
                <p className="line-clamp-3 text-[13px] text-[var(--color-on-surface-variant)]">
                  {corpus.description || pageT("table.noDescription", "No description")}
                </p>
              ),
            },
            {
              id: "ingestProfile",
              header: pageT("table.columns.ingestProfile", "Ingest profile"),
              render: (corpus) => {
                const ingestProfileId = corpus.ingestProfile.trim();
                return ingestProfileId ? (
                  <p className="max-w-[260px] truncate font-mono text-[13px]" title={ingestProfileId}>
                    {ingestProfileId}
                  </p>
                ) : (
                  <span className="text-[13px] text-[var(--color-on-surface-variant)]">
                    {pageT("table.noIngestProfile", "Unbound")}
                  </span>
                );
              },
              width: "240px",
            },
            {
              id: "status",
              header: pageT("table.columns.status", "Status"),
              render: (corpus) => <Badge tone={statusTone(corpus.status)}>{formatRagStatusLabel(corpus.status, t)}</Badge>,
            },
            {
              align: "right",
              cellClassName: "w-24",
              header: pageT("table.columns.actions", "Actions"),
              headerClassName: "w-24",
              id: "actions",
              render: (corpus) => {
                const isMenuOpen = openRowMenu?.code === corpus.code;
                return (
                  <div className="flex justify-end">
                    <Button
                      aria-expanded={isMenuOpen}
                      aria-label={pageT("actions.openRowActions", "Open actions for {{name}}", { name: corpus.name })}
                      data-rag-corpus-menu-trigger={corpus.code}
                      size="icon"
                      type="button"
                      variant="ghost"
                      onClick={(event) => {
                        event.stopPropagation();
                        toggleRowMenu(corpus, event.currentTarget);
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

      {openRowMenu && openMenuCorpus
        ? createPortal(
          <div
            ref={rowMenuRef}
            aria-label={pageT("actions.rowMenuAria", "Actions for {{name}}", { name: openMenuCorpus.name })}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
          >
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuCorpus.code}
              role="menuitem"
              type="button"
              onClick={() => openIngestProfile(openMenuCorpus)}
            >
              {openMenuCorpus.ingestProfile.trim()
                ? pageT("actions.editIngestProfile", "Edit ingest profile")
                : pageT("actions.createIngestProfile", "Create ingest profile")}
            </button>
            {openMenuCorpus.ingestProfile.trim() ? (
              <>
                <button
                  className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                  disabled={rowBusyCode === openMenuCorpus.code}
                  role="menuitem"
                  type="button"
                  onClick={() => {
                    setOpenRowMenu(null);
                    onTestIngest(openMenuCorpus);
                  }}
                >
                  {pageT("actions.testIngest", "Ingest test")}
                </button>
                <button
                  className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                  disabled={rowBusyCode === openMenuCorpus.code}
                  role="menuitem"
                  type="button"
                  onClick={() => {
                    void runRowAction(openMenuCorpus, onDeleteIngestProfile);
                  }}
                >
                  {pageT("actions.deleteIngestProfile", "Delete ingest profile")}
                </button>
              </>
            ) : null}
            <button
              className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuCorpus.code}
              role="menuitem"
              type="button"
              onClick={() => {
                void runRowAction(openMenuCorpus, onDeleteCorpus);
              }}
            >
              {pageT("actions.deleteCorpus", "Delete corpus")}
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}
