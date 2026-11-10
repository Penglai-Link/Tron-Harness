import { FolderTree } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { WorkspaceTree } from "../components/WorkspaceTree";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Field, SelectInput, TextArea, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
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

export function WorkspaceManagementPage() {
  const { t } = useTranslation("pages");
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

  useEffect(() => {
    if (!search.trim() && status === "All") return;
    const ancestors = flatten(visibleTree).filter(record => record.children.length).map(record => record.workspaceCode);
    setExpandedCodes(current => new Set([...current, ...ancestors]));
  }, [search, status, visibleTree]);

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
      toast.error(error instanceof Error ? error.message : t("workspaceManagement.requestFailed"));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void loadTree(); }, [isApiMode, t]);

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
        if (!cancelled) toast.error(error instanceof Error ? error.message : t("workspaceManagement.requestFailed"));
      }
    })();
    return () => { cancelled = true; };
  }, [isApiMode, mode, selectedCode, t]);

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
        const refreshed = isApiMode ? await fetchWorkspaceDetail(createdOrUpdated.workspaceCode) : getWorkspaceDetail(createdOrUpdated.workspaceCode);
        setDetail(refreshed);
        setName(refreshed?.name ?? "");
        setDescription(refreshed?.description ?? "");
        setParentCode(refreshed?.parentWorkspaceCode ?? ROOT_PARENT);
        setMode("edit");
        setSelectedCode(createdOrUpdated.workspaceCode);
      }
      toast.success(t("workspaceManagement.saved"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("workspaceManagement.requestFailed"));
    } finally {
      setSaving(false);
    }
  };

  const save = () => {
    if (!name.trim()) {
      toast.error(t("workspaceManagement.nameRequired"));
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

  return (
    <div className="grid h-full min-h-0 min-w-0 grid-cols-1 overflow-auto lg:grid-cols-[minmax(17rem,22rem)_minmax(0,1fr)] lg:grid-rows-[auto_minmax(0,1fr)] lg:overflow-hidden">
        <aside aria-label={t("workspaceManagement.hierarchy")} className="flex min-h-0 min-w-0 flex-col border-b border-[var(--color-outline-variant)] lg:row-span-2 lg:grid lg:grid-rows-subgrid lg:border-r lg:border-b-0">
          <div className="shrink-0 space-y-4 border-b border-[var(--color-outline-variant)] p-5">
            <div className="flex items-center justify-between gap-3">
              <h2 className="text-sm font-semibold">{t("workspaceManagement.hierarchy")}</h2>
              <Badge variant="secondary" className="tabular-nums">{flatten(visibleTree).length}</Badge>
            </div>
            <div className="grid grid-cols-[minmax(0,1fr)_7rem] gap-2">
              <TextInput className="min-w-0" value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t("workspaceManagement.searchPlaceholder")} aria-label={t("workspaceManagement.searchPlaceholder")} />
              <SelectInput aria-label={t("workspaceManagement.filterByStatus")} className="min-w-0" value={status} onValueChange={(value) => { if (value) setStatus(value as StatusFilter); }}>
                <SelectItem value="All">{t("workspaceManagement.statuses.all")}</SelectItem>
                <SelectItem value="Active">{t("workspaceManagement.statuses.active")}</SelectItem>
                <SelectItem value="Archived">{t("workspaceManagement.statuses.archived")}</SelectItem>
              </SelectInput>
            </div>
          </div>
          <div className="min-h-40 max-h-72 flex-1 overflow-auto overscroll-contain p-3 lg:min-h-0 lg:max-h-none">
            {loading && !tree.length ? <p role="status" className="px-2 py-6 text-center text-sm text-[var(--color-on-surface-variant)]">{t("workspaceManagement.loading")}</p> : visibleTree.length ? <WorkspaceTree records={visibleTree} expandedCodes={expandedCodes} selectedCode={selectedCode}
              label={t("workspaceManagement.hierarchy")} archivedLabel={t("workspaceManagement.statuses.archived")}
              expandLabel={name => t("workspaceManagement.expandNamed", { name })}
              collapseLabel={name => t("workspaceManagement.collapseNamed", { name })}
              createChildLabel={name => t("workspaceManagement.createChildNamed", { name })}
              onToggle={code => setExpandedCodes(current => { const next = new Set(current); if (next.has(code)) next.delete(code); else next.add(code); return next; })}
              onSelect={selectWorkspace} onCreateChild={prepareCreate} /> : <p className="px-2 py-6 text-center text-sm text-[var(--color-on-surface-variant)]">{t("workspaceManagement.empty")}</p>}
          </div>
        </aside>

        <section aria-label={t("workspaceManagement.details")} className="min-h-0 min-w-0 lg:row-span-2 lg:grid lg:grid-rows-subgrid">
          <div className="flex flex-wrap items-center justify-end gap-2 border-b border-[var(--color-outline-variant)] px-5 py-5 sm:px-6">
        <Button variant="secondary" onClick={() => void loadTree()} disabled={loading}>
          {t("workspaceManagement.refresh")}
        </Button>
        <Button onClick={() => prepareCreate(null)}>
          {t("workspaceManagement.createWorkspace")}
        </Button>
          </div>
          <div className="min-h-0 min-w-0 lg:overflow-y-auto lg:overscroll-contain">
          {mode === "create" || detail ? (
            <>
              <div className="p-5 sm:p-6">
              <div className="grid gap-x-5 gap-y-4 xl:grid-cols-2">
                <Field className="min-w-0" label={t("workspaceManagement.name")} required>
                  <TextInput aria-label={t("workspaceManagement.name")} autoFocus={mode === "create"} value={name} onChange={(event) => setName(event.target.value)} />
                </Field>
                <Field className="min-w-0" label={t("workspaceManagement.parent")}>
                  <SelectInput aria-label={t("workspaceManagement.parent")} value={parentCode} onValueChange={(value) => { if (value) setParentCode(value); }}>
                    <SelectItem value={ROOT_PARENT}>{t("workspaceManagement.rootWorkspace")}</SelectItem>
                    {workspaceOptions.filter((item) => item.workspaceCode !== detail?.workspaceCode).map((item) => (
                      <SelectItem key={item.workspaceCode} value={item.workspaceCode}>{item.name} ({item.workspaceCode})</SelectItem>
                    ))}
                  </SelectInput>
                </Field>
                <Field className="min-w-0 xl:col-span-2" label={t("workspaceManagement.descriptionField")}>
                  <TextArea aria-label={t("workspaceManagement.descriptionField")} className="min-h-28 resize-y" value={description} onChange={(event) => setDescription(event.target.value)} />
                </Field>
              </div>

              {detail ? (
                <>
                  <Separator className="my-5" />
                  <dl className="grid grid-cols-[minmax(0,1.4fr)_minmax(4rem,0.6fr)_minmax(5rem,0.8fr)] gap-4 text-sm">
                    <div className="min-w-0"><dt className="mb-1 text-xs text-[var(--color-on-surface-variant)]">{t("workspaceManagement.code")}</dt><dd className="truncate font-mono text-xs" title={detail.workspaceCode}>{detail.workspaceCode}</dd></div>
                    <div><dt className="mb-1 text-xs text-[var(--color-on-surface-variant)]">{t("workspaceManagement.depth")}</dt><dd className="tabular-nums">{detail.depth}</dd></div>
                    <div><dt className="mb-1 text-xs text-[var(--color-on-surface-variant)]">{t("workspaceManagement.children")}</dt><dd className="tabular-nums">{detail.children.length}</dd></div>
                  </dl>
                </>
              ) : null}

              </div>
              <div className="flex flex-wrap items-center gap-2 border-t border-[var(--color-outline-variant)] px-5 py-4 sm:px-6">
                <Button onClick={save} disabled={saving}>{mode === "create" ? t("workspaceManagement.create") : t("workspaceManagement.save")}</Button>
                {mode === "edit" && detail && (detail.parentWorkspaceCode ?? ROOT_PARENT) !== parentCode ? <Button variant="outline" onClick={move} disabled={saving}>{t("workspaceManagement.move")}</Button> : null}
                {mode === "edit" && detail ? (
                  <Button className="ml-auto" variant={detail.status === "Active" ? "destructive" : "outline"} onClick={changeStatus} disabled={saving}>
                    {detail.status === "Active" ? <>{t("workspaceManagement.archive")}</> : <>{t("workspaceManagement.restore")}</>}
                  </Button>
                ) : null}
              </div>
            </>
          ) : (
            <div className="flex min-h-80 flex-1 flex-col items-center justify-center px-6 py-10 text-center">
              <div className="mb-4 flex size-11 items-center justify-center rounded-full bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
                <FolderTree aria-hidden="true" className="size-5" />
              </div>
              <p className="text-sm font-medium">{t("workspaceManagement.selectWorkspace")}</p>
              <p className="mt-1 max-w-xs text-sm text-[var(--color-on-surface-variant)]">{t("workspaceManagement.selectOrCreate")}</p>
            </div>
          )}
          </div>
        </section>
    </div>
  );
}
