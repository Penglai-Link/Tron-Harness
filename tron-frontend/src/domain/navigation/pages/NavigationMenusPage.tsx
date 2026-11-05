import { useSearchParams } from "react-router";
import { Pencil, Trash2, ExternalLink, FilePlus2 } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent } from "react";

import { DataTable } from "@/components/data-table";
import { Field, SelectInput, TextArea, TextInput, ToggleSwitch } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
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
  createNavigationMenu,
  deleteNavigationMenu,
  fetchNavigationAdminMenus,
  updateNavigationMenu,
  type NavigationMenuPayload,
} from "@/domain/navigation/api";
import type { NavigationAdminMenu, NavigationMenuType } from "@/shared/types/Navigation";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { getNavigationIcon, isNavigationIconName } from "@/shared/lib/navigationMenuMapper";
import { useNavigationAdminLocale, useNavigationAdminText } from "./navigationAdminText";

type MenuSourceFilter = "all" | "BuiltIn" | "Custom";
type MenuTypeFilter = "all" | NavigationMenuType;
type EnabledFilter = "all" | "true" | "false";

interface MenuFormState {
  menuCode: string;
  menuType: NavigationMenuType;
  nameZh: string;
  nameEn: string;
  descriptionZh: string;
  descriptionEn: string;
  pageCode: string;
  url: string;
  icon: string;
  enabled: boolean;
}

function emptyMenuForm(): MenuFormState {
  return { menuCode: "", menuType: "Internal", nameZh: "", nameEn: "", descriptionZh: "", descriptionEn: "", pageCode: "", url: "", icon: "", enabled: true };
}

function formFromMenu(menu: NavigationAdminMenu): MenuFormState {
  return { menuCode: menu.menuCode, menuType: menu.menuType, nameZh: menu.translations["zh-CN"], nameEn: menu.translations["en-US"], descriptionZh: menu.description["zh-CN"], descriptionEn: menu.description["en-US"], pageCode: menu.pageCode ?? "", url: menu.url ?? "", icon: menu.icon ?? "", enabled: menu.enabled };
}

function menuPayload(form: MenuFormState): NavigationMenuPayload {
  return {
    menuCode: form.menuCode.trim(),
    menuType: form.menuType,
    translations: { "zh-CN": form.nameZh.trim(), "en-US": form.nameEn.trim() },
    description: { "zh-CN": form.descriptionZh.trim(), "en-US": form.descriptionEn.trim() },
    pageCode: (form.menuType === "Internal" || form.menuType === "Widget") ? form.pageCode.trim() || null : null,
    url: form.menuType === "ExternalLink" ? form.url.trim() || null : null,
    icon: form.icon.trim() || null,
    enabled: form.enabled,
  };
}

function menuTypeLabel(type: NavigationMenuType, text: ReturnType<typeof useNavigationAdminText>) {
  if (type === "Group") return text("types.group", "分组", "Group");
  if (type === "ExternalLink") return text("types.externalLink", "外部链接", "External link");
  if (type === "Widget") return text("types.widget", "面板组件", "Widget");
  return text("types.internal", "内部页面", "Internal page");
}

interface IconPreviewProps {
  emptyLabel: string;
  iconName: string;
  invalidLabel: string;
  previewLabel: string;
}

function IconPreview({ emptyLabel, iconName, invalidLabel, previewLabel }: IconPreviewProps) {
  const validIconName = isNavigationIconName(iconName);
  const Icon = validIconName ? getNavigationIcon(iconName) : null;
  const trimmedIconName = iconName.trim();

  return (
    <div
      aria-label={previewLabel}
      className="flex min-h-9 min-w-9 shrink-0 items-center justify-center gap-2 rounded-lg border border-[var(--color-outline)] px-2 text-[var(--color-on-surface)]"
      data-testid="navigation-icon-preview"
      role="img"
      title={Icon ? trimmedIconName : trimmedIconName ? invalidLabel : emptyLabel}
    >
      {Icon ? <Icon aria-hidden="true" size={18} /> : <span aria-hidden="true" className="text-sm text-[var(--color-on-surface-variant)]">?</span>}
      {trimmedIconName && !Icon ? <span className="max-w-28 truncate text-xs text-[var(--color-on-surface-variant)]">{invalidLabel}</span> : null}
    </div>
  );
}

export function NavigationMenusPage() {
  const [searchParams] = useSearchParams();
  const requestedMenuCode = searchParams.get("menuCode");
  const openedMenuCode = useRef<string | null>(null);
  const text = useNavigationAdminText();
  const locale = useNavigationAdminLocale();
  const toast = useToast();
  const isApiMode = getServiceDataSource("admin") === "api";
  const [records, setRecords] = useState<NavigationAdminMenu[]>([]);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [keyword, setKeyword] = useState(requestedMenuCode ?? "");
  const [source, setSource] = useState<MenuSourceFilter>("all");
  const [menuType, setMenuType] = useState<MenuTypeFilter>("all");
  const [enabled, setEnabled] = useState<EnabledFilter>("all");
  const [loading, setLoading] = useState(false);
  const [editor, setEditor] = useState<MenuFormState | null>(null);
  const [editingCode, setEditingCode] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [updatingCode, setUpdatingCode] = useState<string | null>(null);
  const [deletingCode, setDeletingCode] = useState<string | null>(null);
  const [menuToDelete, setMenuToDelete] = useState<NavigationAdminMenu | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (requestedMenuCode) setKeyword(requestedMenuCode);
  }, [requestedMenuCode]);
  useEffect(() => {
    if (!requestedMenuCode || openedMenuCode.current === requestedMenuCode) return;
    const menu = records.find((item) => item.menuCode === requestedMenuCode);
    if (menu) {
      openedMenuCode.current = requestedMenuCode;
      setEditingCode(menu.menuCode);
      setEditor(formFromMenu(menu));
    }
  }, [records, requestedMenuCode]);

  useEffect(() => {
    if (!isApiMode) {
      setRecords([]);
      setTotal(0);
      return;
    }
    const controller = new AbortController();
    setLoading(true);
    void fetchNavigationAdminMenus({
      current,
      size: pageSize,
      keyword: keyword.trim(),
      source: source === "all" ? undefined : source,
      menuType: menuType === "all" ? undefined : menuType,
      enabled: enabled === "all" ? undefined : enabled === "true",
    }, controller.signal).then((page) => {
      if (controller.signal.aborted) return;
      setRecords(page.records);
      setTotal(page.total);
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadMenus", "菜单定义加载失败", "Menu definitions could not be loaded"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [current, enabled, isApiMode, keyword, menuType, pageSize, refreshKey, source, text, toast]);

  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const startItem = total ? (current - 1) * pageSize + 1 : 0;
  const endItem = total ? Math.min(startItem + records.length - 1, total) : 0;

  function openCreate() {
    setEditingCode(null);
    setEditor(emptyMenuForm());
  }

  function openEdit(menu: NavigationAdminMenu) {
    setEditingCode(menu.menuCode);
    setEditor(formFromMenu(menu));
  }

  async function saveMenu(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editor || !editor.menuCode.trim() || !editor.nameZh.trim() || !editor.nameEn.trim()) {
      toast.error(text("errors.requiredMenu", "请填写菜单编码和中英文名称", "Menu code and both localized names are required"));
      return;
    }
    if (editor.menuType === "Internal" && !editor.pageCode.trim()) {
      toast.error(text("errors.pageCode", "内部页面必须填写 pageCode", "Internal pages require a pageCode"));
      return;
    }
    if (editor.menuType === "Widget" && !editor.pageCode.trim()) {
      toast.error(text("errors.widget", "请选择面板内容", "Select the widget content"));
      return;
    }
    if (editor.menuType === "ExternalLink" && !/^https?:\/\//i.test(editor.url.trim())) {
      toast.error(text("errors.url", "外部链接必须使用 http 或 https", "External links must use http or https"));
      return;
    }

    setSaving(true);
    try {
      const payload = menuPayload(editor);
      if (editingCode) {
        const { menuCode: _menuCode, ...updatePayload } = payload;
        const updated = await updateNavigationMenu(editingCode, updatePayload as Required<Omit<NavigationMenuPayload, "menuCode">>);
        setRecords((items) => items.map((item) => item.menuCode === editingCode ? updated : item));
        toast.success(text("messages.menuUpdated", "菜单已更新", "Menu updated"));
      } else {
        const created = await createNavigationMenu(payload as Required<NavigationMenuPayload>);
        setRecords((items) => [created, ...items]);
        setTotal((value) => value + 1);
        toast.success(text("messages.menuCreated", "菜单已创建", "Menu created"));
      }
      setEditor(null);
      setEditingCode(null);
      setRefreshKey((value) => value + 1);
      window.dispatchEvent(new Event("tron:navigation-updated"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.saveMenu", "菜单保存失败", "Menu could not be saved"));
    } finally {
      setSaving(false);
    }
  }

  async function confirmDeleteMenu() {
    if (!menuToDelete) return;
    setDeletingCode(menuToDelete.menuCode);
    try {
      await deleteNavigationMenu(menuToDelete.menuCode);
      setRecords((items) => items.filter((item) => item.menuCode !== menuToDelete.menuCode));
      setTotal((value) => Math.max(0, value - 1));
      toast.success(text("messages.menuDeleted", "菜单已删除", "Menu deleted"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.deleteMenu", "菜单删除失败", "Menu could not be deleted"));
    } finally {
      setDeletingCode(null);
      setMenuToDelete(null);
    }
  }

  async function toggleMenuEnabled(menu: NavigationAdminMenu) {
    if (menu.source === "BuiltIn" || updatingCode === menu.menuCode) return;

    const enabledValue = !menu.enabled;
    setUpdatingCode(menu.menuCode);
    setRecords((items) => items.map((item) => item.menuCode === menu.menuCode ? { ...item, enabled: enabledValue } : item));
    try {
      const updated = await updateNavigationMenu(menu.menuCode, {
        menuType: menu.menuType,
        translations: menu.translations,
        description: menu.description,
        pageCode: menu.pageCode,
        url: menu.url,
        icon: menu.icon,
        enabled: enabledValue,
      });
      setRecords((items) => items.map((item) => item.menuCode === menu.menuCode ? updated : item));
      toast.success(text("messages.menuUpdated", "菜单已更新", "Menu updated"));
      setRefreshKey((value) => value + 1);
      window.dispatchEvent(new Event("tron:navigation-updated"));
    } catch (error) {
      setRecords((items) => items.map((item) => item.menuCode === menu.menuCode ? menu : item));
      toast.error(error instanceof Error ? error.message : text("errors.updateMenu", "菜单状态更新失败", "Menu status could not be updated"));
    } finally {
      setUpdatingCode(null);
    }
  }

  return (
    <>
      <PageHeader title={text("menus.title", "菜单定义", "Menu Definitions")} actions={<><Button variant="secondary" disabled={loading} onClick={() => setRefreshKey((value) => value + 1)}>{text("actions.refresh", "刷新", "Refresh")}</Button><Button onClick={openCreate}>{text("actions.newMenu", "新建菜单", "New menu")}</Button></>} />

      <ModalDialog open={Boolean(editor)} title={editor ? (editingCode ? text("menus.edit", "编辑菜单", "Edit menu") : text("menus.create", "新建菜单", "New menu")) : ""} className="max-h-[88vh] max-w-2xl overflow-y-auto">
        {editor ? (
          <form className="grid gap-4 lg:grid-cols-2" onSubmit={(event) => void saveMenu(event)}>
                <Field label={text("fields.menuCode", "菜单编码", "Menu code")} required><TextInput value={editor.menuCode} disabled={Boolean(editingCode)} onChange={(event) => setEditor({ ...editor, menuCode: event.target.value })} /></Field>
                <Field label={text("fields.menuType", "菜单类型", "Menu type")} required><SelectInput value={editor.menuType} disabled={editor.menuCode === "" ? false : records.find((menu) => menu.menuCode === editingCode)?.source === "BuiltIn"} onValueChange={(value) => setEditor({ ...editor, menuType: value as NavigationMenuType, pageCode: "", url: "" })}><SelectItem value="Group">{menuTypeLabel("Group", text)}</SelectItem><SelectItem value="Internal">{menuTypeLabel("Internal", text)}</SelectItem><SelectItem value="ExternalLink">{menuTypeLabel("ExternalLink", text)}</SelectItem><SelectItem value="Widget">{menuTypeLabel("Widget", text)}</SelectItem></SelectInput></Field>
                <Field label={text("fields.nameZh", "中文名称", "Chinese name")} required><TextInput value={editor.nameZh} onChange={(event) => setEditor({ ...editor, nameZh: event.target.value })} /></Field>
                <Field label={text("fields.nameEn", "英文名称", "English name")} required><TextInput value={editor.nameEn} onChange={(event) => setEditor({ ...editor, nameEn: event.target.value })} /></Field>
                <Field className="lg:col-span-2" label={text("fields.descriptionZh", "中文描述（选填）", "Chinese description (optional)")}><TextArea rows={3} value={editor.descriptionZh} onChange={(event) => setEditor({ ...editor, descriptionZh: event.target.value })} /></Field>
                <Field className="lg:col-span-2" label={text("fields.descriptionEn", "英文描述（选填）", "English description (optional)")}><TextArea rows={3} value={editor.descriptionEn} onChange={(event) => setEditor({ ...editor, descriptionEn: event.target.value })} /></Field>
                {editor.menuType === "Internal" ? <Field label={text("fields.pageCode", "页面编码", "Page code")} required><TextInput value={editor.pageCode} disabled={records.find((menu) => menu.menuCode === editingCode)?.source === "BuiltIn"} placeholder="agents.index" onChange={(event) => setEditor({ ...editor, pageCode: event.target.value })} /></Field> : null}
                {editor.menuType === "Widget" ? <Field label={text("arrangement.widgetContent", "组件内容", "Widget content")} required><SelectInput value={editor.pageCode} disabled={records.find((menu) => menu.menuCode === editingCode)?.source === "BuiltIn"} onValueChange={(value) => { if (value) setEditor({ ...editor, pageCode: value }); }}><SelectItem value="chat.newSession">{text("arrangement.newSession", "新建会话", "New session")}</SelectItem><SelectItem value="chat.projects">{text("arrangement.projects", "项目列表", "Project list")}</SelectItem><SelectItem value="chat.recents">{text("arrangement.recents", "随聊", "Quick Chats")}</SelectItem></SelectInput></Field> : null}
                {editor.menuType === "ExternalLink" ? <Field label={text("fields.url", "外部 URL", "External URL")} required><TextInput type="url" value={editor.url} placeholder="https://example.com" onChange={(event) => setEditor({ ...editor, url: event.target.value })} /></Field> : null}
                <Field className={editor.menuType === "Group" ? "lg:col-span-2" : undefined} label={<>{text("fields.icon", "图标", "Icon")}<InfoTooltip ariaLabel={text("aria.iconHint", "打开字段说明", "Open field description")}>{text("fields.iconHint", "支持 Lucide 全部图标名称，例如 Cat、cat 或 layout-dashboard。", "Use any Lucide icon name, for example Cat, cat, or layout-dashboard.")}</InfoTooltip><a aria-label={text("aria.iconWebsite", "打开 Lucide 图标官网", "Open Lucide Icons website")} className="inline-flex items-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]" href="https://lucide.dev/icons" rel="noreferrer" target="_blank" title={text("aria.iconWebsite", "打开 Lucide 图标官网", "Open Lucide Icons website")}><ExternalLink aria-hidden="true" size={14} /></a></>}>
                  <div className="flex items-center gap-2">
                    <TextInput className="min-w-0 flex-1" value={editor.icon} placeholder="LayoutDashboard" onChange={(event) => setEditor({ ...editor, icon: event.target.value })} />
                    <IconPreview emptyLabel={text("fields.iconEmpty", "未设置图标", "No icon selected")} iconName={editor.icon} invalidLabel={text("fields.iconInvalid", "未找到图标", "Icon not found")} previewLabel={text("aria.iconPreview", "图标预览", "Icon preview")} />
                  </div>
                </Field>
                <Field className="self-start lg:col-span-2" label={text("fields.enabled", "启用", "Enabled")}>
                  <ToggleSwitch
                    ariaLabel={text("fields.enabled", "启用", "Enabled")}
                    checked={editor.enabled}
                    disabled={records.find((menu) => menu.menuCode === editingCode)?.source === "BuiltIn"}
                    onCheckedChange={(enabledValue) => setEditor({ ...editor, enabled: enabledValue })}
                  />
                </Field>
                <div className="flex justify-end gap-2 lg:col-span-2">
                  <Button type="button" variant="ghost" disabled={saving} onClick={() => setEditor(null)}>{text("actions.cancel", "取消", "Cancel")}</Button>
                  <Button type="submit" disabled={saving} aria-busy={saving}>{saving ? text("actions.saving", "保存中…", "Saving…") : text("actions.save", "保存", "Save")}</Button>
                </div>
              </form>
        ) : null}
      </ModalDialog>

      <DataTable<NavigationAdminMenu>
        rows={records}
        rowKey={(row) => row.menuCode}
        toolbar={<div className="grid w-full grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4"><SearchField className="min-w-0" ariaLabel={text("aria.searchMenus", "搜索菜单", "Search menus")} placeholder={text("filters.searchMenus", "搜索编码或名称…", "Search code or name…")} value={keyword} onChange={(value) => { setKeyword(value); setCurrent(1); }} /><SelectInput className="h-9 w-full min-w-0" aria-label={text("aria.sourceFilter", "按来源筛选", "Filter by source")} value={source} onValueChange={(value) => { if (value) { setSource(value as MenuSourceFilter); setCurrent(1); } }}><SelectItem value="all">{text("filters.allSource", "全部来源", "All sources")}</SelectItem><SelectItem value="BuiltIn">{text("status.builtIn", "内置", "Built-in")}</SelectItem><SelectItem value="Custom">{text("status.custom", "自定义", "Custom")}</SelectItem></SelectInput><SelectInput className="h-9 w-full min-w-0" aria-label={text("aria.typeFilter", "按类型筛选", "Filter by type")} value={menuType} onValueChange={(value) => { if (value) { setMenuType(value as MenuTypeFilter); setCurrent(1); } }}><SelectItem value="all">{text("filters.allTypes", "全部类型", "All types")}</SelectItem><SelectItem value="Group">{menuTypeLabel("Group", text)}</SelectItem><SelectItem value="Internal">{menuTypeLabel("Internal", text)}</SelectItem><SelectItem value="ExternalLink">{menuTypeLabel("ExternalLink", text)}</SelectItem><SelectItem value="Widget">{menuTypeLabel("Widget", text)}</SelectItem></SelectInput><SelectInput className="h-9 w-full min-w-0" aria-label={text("aria.enabledFilter", "按状态筛选", "Filter by status")} value={enabled} onValueChange={(value) => { if (value) { setEnabled(value as EnabledFilter); setCurrent(1); } }}><SelectItem value="all">{text("filters.all", "全部状态", "All statuses")}</SelectItem><SelectItem value="true">{text("status.enabled", "启用", "Enabled")}</SelectItem><SelectItem value="false">{text("status.disabled", "禁用", "Disabled")}</SelectItem></SelectInput></div>}
        emptyState={loading ? text("loadingMenus", "正在加载菜单…", "Loading menus…") : text("menus.empty", "暂无菜单定义。", "No menu definitions found.")}
        pagination={<TablePagination page={current} pageCount={pageCount} pageSize={pageSize} totalItems={total} startItem={startItem} endItem={endItem} itemLabel={text("menus.itemLabel", "个菜单", "menus")} onPageChange={setCurrent} onPageSizeChange={(size) => { setPageSize(size); setCurrent(1); }} />}
        columns={[
          { id: "menu", header: text("columns.menu", "菜单", "Menu"), render: (row) => <div><div className="flex items-center gap-2 font-medium">{row.menuType === "ExternalLink" ? <ExternalLink size={14} aria-hidden="true" /> : row.menuType === "Group" ? <FilePlus2 size={14} aria-hidden="true" /> : null}{row.translations[locale]}</div><div className="font-mono text-xs text-[var(--color-on-surface-variant)]">{row.menuCode}</div>{row.description[locale] ? <p className="mt-1 max-w-sm text-xs text-[var(--color-on-surface-variant)]">{row.description[locale]}</p> : null}</div> },
          { id: "type", header: text("columns.type", "类型", "Type"), render: (row) => menuTypeLabel(row.menuType, text) },
          { id: "source", header: text("columns.source", "来源", "Source"), render: (row) => <Badge variant={row.source === "BuiltIn" ? "info" : "default"}>{row.source === "BuiltIn" ? text("status.builtIn", "内置", "Built-in") : text("status.custom", "自定义", "Custom")}</Badge> },
          { id: "target", header: text("columns.target", "目标", "Target"), render: (row) => <span className="max-w-52 truncate font-mono text-xs">{row.pageCode ?? row.url ?? "—"}</span> },
          { id: "status", header: text("columns.status", "状态", "Status"), headerClassName: "text-center", cellClassName: "text-center", render: (row) => <div className="flex justify-center"><ToggleSwitch ariaLabel={text("aria.toggleMenu", row.enabled ? `禁用 ${row.menuCode}` : `启用 ${row.menuCode}`, row.enabled ? `Disable ${row.menuCode}` : `Enable ${row.menuCode}`)} checked={row.enabled} disabled={row.source === "BuiltIn" || updatingCode === row.menuCode} onCheckedChange={() => void toggleMenuEnabled(row)} /></div> },
          { id: "actions", header: text("columns.actions", "操作", "Actions"), align: "right", render: (row) => <div className="flex justify-end gap-1"><Button size="icon" variant="ghost" aria-label={text("aria.editMenu", `编辑 ${row.menuCode}`, `Edit ${row.menuCode}`)} onClick={() => openEdit(row)}><Pencil size={16} aria-hidden="true" /></Button>{row.source === "Custom" ? <Button size="icon" variant="ghost" aria-label={text("aria.deleteMenu", `删除 ${row.menuCode}`, `Delete ${row.menuCode}`)} disabled={deletingCode === row.menuCode} onClick={() => setMenuToDelete(row)}><Trash2 size={16} aria-hidden="true" /></Button> : null}</div> },
        ]}
      />

      <ConfirmDialog open={Boolean(menuToDelete)} title={text("menus.deleteTitle", "删除菜单", "Delete menu")} description={text("menus.deleteDescription", `确定删除 ${menuToDelete?.menuCode ?? ""}？只有未被任何领域引用的自定义菜单可以删除。`, `Delete ${menuToDelete?.menuCode ?? ""}? Only custom menus not referenced by any domain can be deleted.`)} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.delete", "删除", "Delete")} pending={Boolean(deletingCode)} onCancel={() => setMenuToDelete(null)} onConfirm={() => void confirmDeleteMenu()} />
    </>
  );
}
