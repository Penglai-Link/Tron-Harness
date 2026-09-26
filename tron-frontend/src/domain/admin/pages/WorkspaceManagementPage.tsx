import { useEffect, useMemo, useState } from "react";
import { ChevronDown, ChevronRight, FolderTree, Plus, RefreshCcw, Save, Trash2, RotateCcw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import {
  archiveWorkspace,
  createWorkspace,
  fetchWorkspaceDetail,
  fetchWorkspaceTree,
  moveWorkspace,
  restoreWorkspace,
  updateWorkspace,
} from "@/domain/admin/api";
import {
  archiveWorkspaceRecord,
  createWorkspaceRecord,
  getWorkspaceDetail,
  getWorkspaceTree,
  moveWorkspaceRecord,
  restoreWorkspaceRecord,
  updateWorkspaceRecord,
} from "@/domain/admin/mock";
import type { WorkspaceDetailRecord, WorkspaceStatus, WorkspaceTreeNodeRecord } from "@/domain/admin/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

type StatusFilter = WorkspaceStatus | "All";

const ROOT_PARENT = "__ROOT__";

function flatten(records: WorkspaceTreeNodeRecord[]): WorkspaceTreeNodeRecord[] {
  return records.flatMap((record) => [record, ...flatten(record.children)]);
}

function filterTree(records: WorkspaceTreeNodeRecord[], search: string, status: StatusFilter): WorkspaceTreeNodeRecord[] {
  const normalizedSearch = search.trim().toLowerCase();
  return records.flatMap((record) => {
    const children = filterTree(record.children, search, status);
    const matchesSearch = !normalizedSearch || [record.workspaceCode, record.name, record.description ?? ""].join(" ").toLowerCase().includes(normalizedSearch);
    const matchesStatus = status === "All" || record.status === status;
    return (matchesSearch && matchesStatus) || children.length ? [{ ...record, children }] : [];
  });
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Workspace request failed.";
}

function TreeNode({
  record,
  expanded,
  selected,
  onToggle,
  onSelect,
  onCreateChild,
}: {
  record: WorkspaceTreeNodeRecord;
  expanded: boolean;
  selected: boolean;
  onToggle: () => void;
  onSelect: () => void;
  onCreateChild: () => void;
}) {
  return (
    <div>
      <div className={`flex items-center gap-1 rounded-lg px-2 py-1.5 ${selected ? "bg-[var(--color-surface-container-high)]" : ""}`}>
        <button type="button" className="p-1" onClick={onToggle} aria-label={expanded ? "Collapse" : "Expand"}>
          {record.children.length ? (expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />) : <span className="inline-block w-[15px]" />}
        </button>
        <button type="button" className="min-w-0 flex-1 truncate text-left text-sm" onClick={onSelect}>
          <span className="font-medium">{record.name}</span>
          <span className="ml-2 text-xs text-[var(--color-on-surface-variant)]">{record.workspaceCode}</span>
        </button>
        <button type="button" className="rounded p-1 text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface-container)]" onClick={onCreateChild} aria-label="Create child workspace">
          <Plus size={14} />
        </button>
      </div>
    </div>
  );
}

export function WorkspaceManagementPage() {
  const toast = useToast();
  const isApiMode = getServiceDataSource("admin") === "api";
  const [tree, setTree] = useState<WorkspaceTreeNodeRecord[]>([]);
  const [detail, setDetail] = useState<WorkspaceDetailRecord | null>(null);
  const [selectedCode, setSelectedCode] = useState<string | null>(null);
  const [expandedCodes, setExpandedCodes] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<StatusFilter>("All");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [mode, setMode] = useState<"edit" | "create">("edit");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [parentCode, setParentCode] = useState(ROOT_PARENT);

  const visibleTree = useMemo(() => filterTree(tree, search, status), [tree, search, status]);
  const workspaceOptions = useMemo(() => flatten(tree).filter((record) => record.status === "Active"), [tree]);

  const loadTree = async () => {
    setLoading(true);
    try {
      const nextTree = isApiMode
        ? await fetchWorkspaceTree({ includeArchived: true })
        : getWorkspaceTree();
      setTree(nextTree);
      setExpandedCodes((current) => current.size ? current : new Set(nextTree.filter((record) => record.children.length).map((record) => record.workspaceCode)));
      if (!selectedCode && nextTree[0]) setSelectedCode(nextTree[0].workspaceCode);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void loadTree(); }, [isApiMode]);

  useEffect(() => {
    if (!selectedCode || mode === "create") return;
    let cancelled = false;
    (async () => {
      try {
        const nextDetail = isApiMode ? await fetchWorkspaceDetail(selectedCode) : getWorkspaceDetail(selectedCode);
        if (cancelled) return;
        setDetail(nextDetail);
        setName(nextDetail?.name ?? "");
        setDescription(nextDetail?.description ?? "");
        setParentCode(nextDetail?.parentWorkspaceCode ?? ROOT_PARENT);
      } catch (error) {
        if (!cancelled) toast.error(errorMessage(error));
      }
    })();
    return () => { cancelled = true; };
  }, [isApiMode, mode, selectedCode]);

  const selectWorkspace = (workspaceCode: string) => {
    setMode("edit");
    setSelectedCode(workspaceCode);
  };

  const prepareCreate = (parentWorkspaceCode: string | null) => {
    setMode("create");
    setDetail(null);
    setSelectedCode(null);
    setName("");
    setDescription("");
    setParentCode(parentWorkspaceCode ?? ROOT_PARENT);
  };

  const runMutation = async (action: () => Promise<WorkspaceDetailRecord | unknown>) => {
    setSaving(true);
    try {
      const result = await action();
      const createdOrUpdated = result as WorkspaceDetailRecord;
      await loadTree();
      if (createdOrUpdated?.workspaceCode) {
        setMode("edit");
        setSelectedCode(createdOrUpdated.workspaceCode);
      }
      toast.success("Workspace saved.");
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setSaving(false);
    }
  };

  const save = () => {
    if (!name.trim()) {
      toast.error("Workspace name is required.");
      return;
    }
    if (mode === "create") {
      void runMutation(() => isApiMode
        ? createWorkspace({ name, description, parentWorkspaceCode: parentCode === ROOT_PARENT ? null : parentCode })
        : Promise.resolve(createWorkspaceRecord({ name, description, parentWorkspaceCode: parentCode === ROOT_PARENT ? null : parentCode })));
      return;
    }
    if (!detail) return;
    void runMutation(() => isApiMode
      ? updateWorkspace(detail.workspaceCode, { name, description })
      : Promise.resolve(updateWorkspaceRecord(detail.workspaceCode, { name, description })));
  };

  const move = () => {
    if (!detail) return;
    void runMutation(() => isApiMode
      ? moveWorkspace(detail.workspaceCode, { parentWorkspaceCode: parentCode === ROOT_PARENT ? null : parentCode })
      : Promise.resolve(moveWorkspaceRecord(detail.workspaceCode, parentCode === ROOT_PARENT ? null : parentCode)));
  };

  const changeStatus = () => {
    if (!detail) return;
    const action = detail.status === "Active"
      ? (isApiMode ? () => archiveWorkspace(detail.workspaceCode) : () => Promise.resolve(archiveWorkspaceRecord(detail.workspaceCode)))
      : (isApiMode ? () => restoreWorkspace(detail.workspaceCode) : () => Promise.resolve(restoreWorkspaceRecord(detail.workspaceCode)));
    void runMutation(action);
  };

  const renderTree = (records: WorkspaceTreeNodeRecord[]) => records.map((record) => (
    <div key={record.workspaceCode}>
      <TreeNode
        record={record}
        expanded={expandedCodes.has(record.workspaceCode)}
        selected={selectedCode === record.workspaceCode}
        onToggle={() => setExpandedCodes((current) => {
          const next = new Set(current);
          if (next.has(record.workspaceCode)) next.delete(record.workspaceCode); else next.add(record.workspaceCode);
          return next;
        })}
        onSelect={() => selectWorkspace(record.workspaceCode)}
        onCreateChild={() => prepareCreate(record.workspaceCode)}
      />
      {expandedCodes.has(record.workspaceCode) && record.children.length ? (
        <div className="ml-4 border-l border-[var(--color-outline-variant)] pl-2">{renderTree(record.children)}</div>
      ) : null}
    </div>
  ));

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Workspaces</h1>
          <p className="text-sm text-[var(--color-on-surface-variant)]">Manage workspace hierarchy and lifecycle.</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => void loadTree()} disabled={loading}><RefreshCcw size={14} />Refresh</Button>
          <Button size="sm" onClick={() => prepareCreate(null)}><Plus size={14} />Create workspace</Button>
        </div>
      </div>
      <div className="flex min-h-0 flex-1 gap-4">
        <section data-theme-anchor="panel" className="w-[34%] min-w-[280px] overflow-auto rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-3">
          <div className="mb-3 flex gap-2">
            <input className="min-w-0 flex-1 rounded-md border border-[var(--color-outline-variant)] bg-transparent px-3 py-2 text-sm" value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search workspaces" />
            <select className="rounded-md border border-[var(--color-outline-variant)] bg-transparent px-2 text-sm" value={status} onChange={(event) => setStatus(event.target.value as StatusFilter)}>
              <option value="All">All</option><option value="Active">Active</option><option value="Archived">Archived</option>
            </select>
          </div>
          {visibleTree.length ? renderTree(visibleTree) : <p className="p-4 text-sm text-[var(--color-on-surface-variant)]">No workspaces.</p>}
        </section>
        <section data-theme-anchor="panel" className="min-w-0 flex-1 overflow-auto rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-5">
          <div className="mb-5 flex items-center gap-2"><FolderTree size={18} /><h2 className="text-lg font-semibold">{mode === "create" ? "Create workspace" : detail?.name ?? "Select a workspace"}</h2></div>
          {mode === "create" || detail ? (
            <div className="grid max-w-2xl gap-4">
              <label className="grid gap-1 text-sm font-medium">Name<input className="rounded-md border border-[var(--color-outline-variant)] bg-transparent px-3 py-2 font-normal" value={name} onChange={(event) => setName(event.target.value)} /></label>
              <label className="grid gap-1 text-sm font-medium">Description<textarea className="min-h-24 rounded-md border border-[var(--color-outline-variant)] bg-transparent px-3 py-2 font-normal" value={description} onChange={(event) => setDescription(event.target.value)} /></label>
              <label className="grid gap-1 text-sm font-medium">Parent<select className="rounded-md border border-[var(--color-outline-variant)] bg-transparent px-3 py-2 font-normal" value={parentCode} onChange={(event) => setParentCode(event.target.value)}><option value={ROOT_PARENT}>Root workspace</option>{workspaceOptions.filter((item) => item.workspaceCode !== detail?.workspaceCode).map((item) => <option key={item.workspaceCode} value={item.workspaceCode}>{item.name} ({item.workspaceCode})</option>)}</select></label>
              <div className="flex flex-wrap gap-2">
                <Button onClick={save} disabled={saving}><Save size={14} />{mode === "create" ? "Create" : "Save"}</Button>
                {mode === "edit" && detail && detail.parentWorkspaceCode !== parentCode ? <Button variant="outline" onClick={move} disabled={saving}>Move</Button> : null}
                {mode === "edit" && detail ? <Button variant="outline" onClick={changeStatus} disabled={saving}>{detail.status === "Active" ? <><Trash2 size={14} />Archive</> : <><RotateCcw size={14} />Restore</>}</Button> : null}
              </div>
              {detail ? <dl className="grid grid-cols-2 gap-3 border-t border-[var(--color-outline-variant)] pt-4 text-sm"><div><dt className="text-[var(--color-on-surface-variant)]">Code</dt><dd>{detail.workspaceCode}</dd></div><div><dt className="text-[var(--color-on-surface-variant)]">Status</dt><dd>{detail.status}</dd></div><div><dt className="text-[var(--color-on-surface-variant)]">Depth</dt><dd>{detail.depth}</dd></div><div><dt className="text-[var(--color-on-surface-variant)]">Children</dt><dd>{detail.children.length}</dd></div></dl> : null}
            </div>
          ) : <p className="text-sm text-[var(--color-on-surface-variant)]">Select a workspace or create one.</p>}
        </section>
      </div>
    </div>
  );
}
