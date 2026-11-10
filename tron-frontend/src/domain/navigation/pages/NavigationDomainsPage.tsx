import { ArrowDown, ArrowUp, MoreHorizontal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { useNavigate } from "react-router";

import { DataTable } from "@/components/data-table";
import { Field, SelectInput, TextArea, TextInput, ToggleSwitch } from "@/components/forms";
import { ModalDialog } from "@/components/modal-dialog";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
} from "@/shared";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SelectItem } from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  createNavigationDomain,
  deleteNavigationDomain,
  fetchNavigationAdminDomains,
  updateNavigationDomain,
  type NavigationDomainPayload,
} from "@/domain/navigation/api";
import type { NavigationAdminDomain } from "@/shared/types/Navigation";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useNavigationAdminLocale, useNavigationAdminText } from "./navigationAdminText";

type EnabledFilter = "all" | "true" | "false";

interface DomainFormState {
  nameZh: string;
  nameEn: string;
  descriptionZh: string;
  descriptionEn: string;
  enabled: boolean;
  isDefault: boolean;
  position: string;
}

function emptyDomainForm(): DomainFormState {
  return {
    nameZh: "",
    nameEn: "",
    descriptionZh: "",
    descriptionEn: "",
    enabled: true,
    isDefault: false,
    position: "0",
  };
}

function formFromDomain(domain: NavigationAdminDomain): DomainFormState {
  return {
    nameZh: domain.name["zh-CN"],
    nameEn: domain.name["en-US"],
    descriptionZh: domain.description["zh-CN"],
    descriptionEn: domain.description["en-US"],
    enabled: domain.enabled,
    isDefault: domain.isDefault,
    position: String(domain.position),
  };
}

function domainPayload(form: DomainFormState): NavigationDomainPayload {
  return {
    name: { "zh-CN": form.nameZh.trim(), "en-US": form.nameEn.trim() },
    description: { "zh-CN": form.descriptionZh.trim(), "en-US": form.descriptionEn.trim() },
    enabled: form.enabled,
    isDefault: form.isDefault,
    position: Number(form.position) || 0,
  };
}

function domainUpdatePayload(domain: NavigationAdminDomain, position = domain.position): Omit<Required<NavigationDomainPayload>, "domainCode"> {
  return {
    name: domain.name,
    description: domain.description,
    enabled: domain.enabled,
    isDefault: domain.isDefault,
    position,
  };
}

export function NavigationDomainsPage() {
  const text = useNavigationAdminText();
  const locale = useNavigationAdminLocale();
  const toast = useToast();
  const navigate = useNavigate();
  const isApiMode = getServiceDataSource("admin") === "api";
  const [records, setRecords] = useState<NavigationAdminDomain[]>([]);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [keyword, setKeyword] = useState("");
  const [enabledFilter, setEnabledFilter] = useState<EnabledFilter>("all");
  const [loading, setLoading] = useState(false);
  const [editor, setEditor] = useState<DomainFormState | null>(null);
  const [editingCode, setEditingCode] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingCode, setDeletingCode] = useState<string | null>(null);
  const [reorderingCode, setReorderingCode] = useState<string | null>(null);
  const [domainToDelete, setDomainToDelete] = useState<NavigationAdminDomain | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (!isApiMode) {
      setRecords([]);
      setTotal(0);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    void fetchNavigationAdminDomains({
      current,
      size: pageSize,
      keyword: keyword.trim(),
      enabled: enabledFilter === "all" ? undefined : enabledFilter === "true",
    }, controller.signal).then((page) => {
      if (controller.signal.aborted) return;
      setRecords(page.records);
      setTotal(page.total);
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadDomains", "领域列表加载失败", "Domains could not be loaded"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [current, enabledFilter, isApiMode, keyword, pageSize, refreshKey, text, toast]);

  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const startItem = total ? (current - 1) * pageSize + 1 : 0;
  const endItem = total ? Math.min(startItem + records.length - 1, total) : 0;
  const editingDomain = useMemo(
    () => (editingCode ? records.find((domain) => domain.domainCode === editingCode) ?? null : null),
    [editingCode, records],
  );

  function openCreate() {
    setEditingCode(null);
    setEditor(emptyDomainForm());
  }

  function openEdit(domain: NavigationAdminDomain) {
    setEditingCode(domain.domainCode);
    setEditor(formFromDomain(domain));
  }

  async function saveDomain(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editor || !editor.nameZh.trim() || !editor.nameEn.trim()) {
      toast.error(text("errors.required", "请填写中英文名称", "Both localized names are required"));
      return;
    }

    setSaving(true);
    try {
      const payload = domainPayload(editor);
      if (editingCode) {
        const updatePayload = payload;
        const updated = await updateNavigationDomain(editingCode, updatePayload as Required<Omit<NavigationDomainPayload, "domainCode">>);
        setRecords((items) => items.map((item) => item.domainCode === editingCode ? updated : item));
        toast.success(text("messages.domainUpdated", "领域已更新", "Domain updated"));
      } else {
        const { position: _position, ...createPayload } = payload;
        const created = await createNavigationDomain(createPayload as Omit<Required<NavigationDomainPayload>, "position" | "domainCode">);
        setRecords((items) => [...items, created]);
        setTotal((value) => value + 1);
        toast.success(text("messages.domainCreated", "领域已创建", "Domain created"));
      }
      setEditor(null);
      setEditingCode(null);
      setRefreshKey((value) => value + 1);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.saveDomain", "领域保存失败", "Domain could not be saved"));
    } finally {
      setSaving(false);
    }
  }

  async function confirmDeleteDomain() {
    if (!domainToDelete) return;
    setDeletingCode(domainToDelete.domainCode);
    try {
      await deleteNavigationDomain(domainToDelete.domainCode);
      setRecords((items) => items.filter((item) => item.domainCode !== domainToDelete.domainCode));
      setTotal((value) => Math.max(0, value - 1));
      toast.success(text("messages.domainDeleted", "领域已删除", "Domain deleted"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.deleteDomain", "领域删除失败", "Domain could not be deleted"));
    } finally {
      setDeletingCode(null);
      setDomainToDelete(null);
    }
  }

  async function moveDomain(domain: NavigationAdminDomain, direction: -1 | 1) {
    if (reorderingCode) return;

    setReorderingCode(domain.domainCode);
    try {
      const allDomains = await fetchNavigationAdminDomains({ current: 1, size: 200, keyword: "" });
      const orderedDomains = [...allDomains.records].sort((left, right) => left.position - right.position || left.domainCode.localeCompare(right.domainCode));
      const index = orderedDomains.findIndex((item) => item.domainCode === domain.domainCode);
      const neighbor = index >= 0 ? orderedDomains[index + direction] : undefined;
      if (!neighbor) return;

      await updateNavigationDomain(domain.domainCode, domainUpdatePayload(domain, neighbor.position));
      await updateNavigationDomain(neighbor.domainCode, domainUpdatePayload(neighbor, domain.position));

      setRecords((items) => items.map((item) => {
        if (item.domainCode === domain.domainCode) return { ...item, position: neighbor.position };
        if (item.domainCode === neighbor.domainCode) return { ...item, position: domain.position };
        return item;
      }).sort((left, right) => left.position - right.position || left.domainCode.localeCompare(right.domainCode)));
      toast.success(text("messages.domainReordered", "领域顺序已更新", "Domain order updated"));
      setRefreshKey((value) => value + 1);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.reorderDomain", "领域排序失败", "Domain order could not be updated"));
      setRefreshKey((value) => value + 1);
    } finally {
      setReorderingCode(null);
    }
  }

  return (
    <>
      <PageHeader
        title={text("domains.title", "菜单领域", "Navigation Domains")}
        actions={<>
          <Button variant="secondary" disabled={loading} onClick={() => setRefreshKey((value) => value + 1)}>{text("actions.refresh", "刷新", "Refresh")}</Button>
          <Button onClick={openCreate}>{text("actions.newDomain", "新建领域", "New domain")}</Button>
        </>}
      />

      <ModalDialog open={Boolean(editor)} onClose={() => { if (!saving) setEditor(null); }} title={editor ? (editingDomain ? text("domains.edit", "编辑领域", "Edit domain") : text("domains.create", "新建领域", "New domain")) : ""} className="max-h-[88vh] sm:max-w-2xl overflow-y-auto">
        {editor ? (
          <form className="grid gap-4 lg:grid-cols-2" onSubmit={(event) => void saveDomain(event)}>
                <Field label={text("fields.nameZh", "中文名称", "Chinese name")} required><TextInput value={editor.nameZh} onChange={(event) => setEditor({ ...editor, nameZh: event.target.value })} /></Field>
                <Field label={text("fields.nameEn", "英文名称", "English name")} required><TextInput value={editor.nameEn} onChange={(event) => setEditor({ ...editor, nameEn: event.target.value })} /></Field>
                <Field label={text("fields.descriptionZh", "中文描述", "Chinese description")}><TextArea value={editor.descriptionZh} onChange={(event) => setEditor({ ...editor, descriptionZh: event.target.value })} /></Field>
                <Field label={text("fields.descriptionEn", "英文描述", "English description")}><TextArea value={editor.descriptionEn} onChange={(event) => setEditor({ ...editor, descriptionEn: event.target.value })} /></Field>
                <div className="flex flex-wrap items-center gap-5 lg:col-span-2"><ToggleSwitch checked={editor.enabled} disabled={editingCode === "ADVANCED" || Boolean(editingDomain?.providerPackId)} label={text("fields.enabled", "启用", "Enabled")} onCheckedChange={(enabled) => setEditor({ ...editor, enabled })} /><ToggleSwitch checked={editor.isDefault} label={text("fields.default", "默认领域", "Default domain")} onCheckedChange={(isDefault) => setEditor({ ...editor, isDefault })} /></div>
                <div className="flex justify-end gap-2 lg:col-span-2">
                  <Button type="button" variant="ghost" disabled={saving} onClick={() => setEditor(null)}>{text("actions.cancel", "取消", "Cancel")}</Button>
                  <Button type="submit" disabled={saving} aria-busy={saving}>{saving ? text("actions.saving", "保存中…", "Saving…") : text("actions.save", "保存", "Save")}</Button>
                </div>
              </form>
        ) : null}
      </ModalDialog>

      <DataTable<NavigationAdminDomain>
        rows={records}
        rowKey={(row) => row.domainCode}
        toolbar={
          <div className="grid w-full grid-cols-1 gap-2 sm:grid-cols-2">
            <SearchField
              className="w-full min-w-0"
              ariaLabel={text("aria.searchDomains", "搜索领域", "Search domains")}
              placeholder={text("filters.search", "搜索领域编码或名称…", "Search domain code or name…")}
              value={keyword}
              onChange={(value) => { setKeyword(value); setCurrent(1); }}
            />
            <SelectInput
              className="h-9 w-full min-w-0"
              aria-label={text("aria.enabledFilter", "按状态筛选", "Filter by status")}
              value={enabledFilter}
              onValueChange={(value) => { if (value) { setEnabledFilter(value as EnabledFilter); setCurrent(1); } }}
            >
              <SelectItem value="all">{text("filters.all", "全部状态", "All statuses")}</SelectItem>
              <SelectItem value="true">{text("filters.enabled", "已启用", "Enabled")}</SelectItem>
              <SelectItem value="false">{text("filters.disabled", "已禁用", "Disabled")}</SelectItem>
            </SelectInput>
          </div>
        }
        emptyState={loading ? text("loading", "正在加载领域…", "Loading domains…") : text("domains.empty", "暂无领域。", "No domains found.")}
        pagination={<TablePagination page={current} pageCount={pageCount} pageSize={pageSize} totalItems={total} startItem={startItem} endItem={endItem} itemLabel={text("domains.itemLabel", "个领域", "domains")} onPageChange={setCurrent} onPageSizeChange={(size) => { setPageSize(size); setCurrent(1); }} />}
        columns={[
          { id: "domain", header: text("columns.domain", "领域", "Domain"), render: (row) => <div><div className="font-medium">{row.name[locale]}</div><div className="font-mono text-xs text-[var(--color-on-surface-variant)]">{row.domainCode}</div></div> },
          { id: "status", header: text("columns.status", "状态", "Status"), render: (row) => <div className="flex flex-wrap gap-1.5"><Badge variant={row.enabled ? "success" : "default"}>{row.enabled ? text("status.enabled", "启用", "Enabled") : text("status.disabled", "禁用", "Disabled")}</Badge>{row.builtIn ? <Badge variant="info">{text("status.builtIn", "内置", "Built-in")}</Badge> : null}{row.providerPackId ? <Badge>{text("status.capabilityPack", "能力包", "Capability pack")} · {row.providerPackId}</Badge> : null}{row.isDefault ? <Badge variant="warning">{text("status.default", "默认", "Default")}</Badge> : null}</div> },
          {
            id: "actions",
            header: text("columns.actions", "操作", "Actions"),
            align: "right",
            render: (row) => {
              const rowIndex = records.findIndex((item) => item.domainCode === row.domainCode);
              const reordering = reorderingCode !== null;
              return (
                <div className="flex justify-end gap-1">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={text("aria.moveDomainUp", `上移 ${row.domainCode}`, `Move ${row.domainCode} up`)}
                    title={text("aria.moveDomainUp", `上移 ${row.domainCode}`, `Move ${row.domainCode} up`)}
                    disabled={reordering || rowIndex <= 0}
                    onClick={() => void moveDomain(row, -1)}
                  >
                    <ArrowUp size={16} aria-hidden="true" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={text("aria.moveDomainDown", `下移 ${row.domainCode}`, `Move ${row.domainCode} down`)}
                    title={text("aria.moveDomainDown", `下移 ${row.domainCode}`, `Move ${row.domainCode} down`)}
                    disabled={reordering || rowIndex < 0 || rowIndex >= records.length - 1}
                    onClick={() => void moveDomain(row, 1)}
                  >
                    <ArrowDown size={16} aria-hidden="true" />
                  </Button>
                  <DropdownMenu>
                    <DropdownMenuTrigger
                      render={
                        <Button
                          size="icon"
                          variant="ghost"
                          aria-label={text("aria.domainActions", `打开 ${row.domainCode} 操作`, `Open actions for ${row.domainCode}`)}
                        >
                          <MoreHorizontal size={16} aria-hidden="true" />
                        </Button>
                      }
                    />
                    <DropdownMenuContent align="end" className="w-36">
                      <DropdownMenuItem onClick={() => openEdit(row)}>{text("actions.edit", "编辑", "Edit")}</DropdownMenuItem>
                      <DropdownMenuItem onClick={() => navigate(`/admin/navigation/menu-tree?treeCode=${encodeURIComponent(row.domainCode)}`)}>{text("actions.arrange", "编排菜单", "Arrange menu")}</DropdownMenuItem>
                      {!row.builtIn && !row.providerPackId ? (
                        <>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem variant="destructive" disabled={deletingCode === row.domainCode} onClick={() => setDomainToDelete(row)}>{text("actions.delete", "删除", "Delete")}</DropdownMenuItem>
                        </>
                      ) : null}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              );
            },
          },
        ]}
      />

      <ConfirmDialog open={Boolean(domainToDelete)} title={text("domains.deleteTitle", "删除领域", "Delete domain")} description={text("domains.deleteDescription", `确定删除 ${domainToDelete?.domainCode ?? ""}？只有没有菜单编排节点的自定义领域可以删除。`, `Delete ${domainToDelete?.domainCode ?? ""}? Only custom domains without menu items can be deleted.`)} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.delete", "删除", "Delete")} pending={Boolean(deletingCode)} onCancel={() => setDomainToDelete(null)} onConfirm={() => void confirmDeleteDomain()} />
    </>
  );
}
