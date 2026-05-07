import {
  useMemo,
  useState,
} from "react";
import {
  ChevronLeft,
  ChevronRight,
  Database,
  Filter,
  Plus,
  Settings2,
  Terminal,
  Trash2,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  SelectInput,
  TextInput,
  TronShell,
} from "@/shared";

import {
  useApps,
  type AppRecord,
} from "@/domain/app/api";

function IconButton({ label, children, onClick }: { label: string; children: React.ReactNode; onClick?: () => void }) {
  return (
    <button className="inline-flex size-8 items-center justify-center rounded-lg text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]" type="button" aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function tagList(tags: string[]) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <Badge key={tag} tone="neutral">
          {tag}
        </Badge>
      ))}
    </div>
  );
}

export function AppsPage() {
  const rows = useApps();
  const [query, setQuery] = useState("");
  const [source, setSource] = useState("all");
  const [page, setPage] = useState(1);
  const [notice, setNotice] = useState("No app action selected.");
  const filteredRows = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return rows.filter((row) => (source === "all" || row.source === source) && (!normalized || Object.values(row).join(" ").toLowerCase().includes(normalized)));
  }, [query, rows, source]);

  return (
    <TronShell activeItem="Apps" searchPlaceholder="Search apps, paths, worktrees...">
      <PageHeader
        title="Connected Applications"
        description="Manage application sources, paths, worktrees, tags, and quick actions."
        status={`${filteredRows.length} visible`}
        actions={
          <>
            <Button variant="secondary" onClick={() => setNotice("Import Apps requested")}>
              <Database size={15} />
              Import
            </Button>
            <Button onClick={() => setNotice("New App requested")}>
              <Plus size={15} />
              New App
            </Button>
          </>
        }
      />
      <div className="mb-4">
        <Notice value={notice} />
      </div>
      <DataTable<AppRecord>
        title="Application Registry"
        rows={filteredRows}
        rowKey={(row) => row.id}
        toolbar={
          <>
            <TextInput className="h-8 w-56" placeholder="Search apps..." value={query} onChange={(event) => setQuery(event.target.value)} />
            <SelectInput className="h-8 w-40" value={source} onChange={(event) => setSource(event.target.value)}>
              <option value="all">All sources</option>
              <option value="Frontend">Frontend</option>
              <option value="Backend">Backend</option>
              <option value="Worker">Worker</option>
            </SelectInput>
            <Button size="sm" variant="secondary" onClick={() => setNotice("Sort by last update toggled")}>
              <Filter size={14} />
              Sort
            </Button>
          </>
        }
        pagination={
          <div className="flex w-full items-center justify-between">
            <span>
              Page {page} of 2 · {filteredRows.length} matching apps
            </span>
            <div className="flex gap-2">
              <Button size="sm" variant="secondary" disabled={page === 1} onClick={() => setPage(1)}>
                <ChevronLeft size={14} />
                Previous
              </Button>
              <Button size="sm" variant="secondary" disabled={page === 2} onClick={() => setPage(2)}>
                Next
                <ChevronRight size={14} />
              </Button>
            </div>
          </div>
        }
        columns={[
          { id: "name", header: "App Name", render: (row) => <span className="font-medium">{row.name}</span> },
          { id: "source", header: "Source", render: (row) => <Badge tone="accent">{row.source}</Badge> },
          { id: "path", header: "Path", render: (row) => <span className="font-mono text-[13px]">{row.path}</span> },
          { id: "worktree", header: "Worktree", render: (row) => <span>{row.worktree}</span> },
          { id: "tags", header: "Tags", render: (row) => tagList(row.tags) },
          {
            id: "actions",
            header: "Actions",
            align: "right",
            render: (row) => (
              <div className="flex justify-end gap-1">
                <IconButton label={`Edit ${row.name}`} onClick={() => setNotice(`Edit ${row.name}`)}>
                  <Settings2 size={15} />
                </IconButton>
                <IconButton label={`Open terminal for ${row.name}`} onClick={() => setNotice(`Terminal for ${row.name}`)}>
                  <Terminal size={15} />
                </IconButton>
                <IconButton label={`Delete ${row.name}`} onClick={() => setNotice(`Delete ${row.name} requested`)}>
                  <Trash2 size={15} />
                </IconButton>
              </div>
            ),
          },
        ]}
      />
    </TronShell>
  );
}
