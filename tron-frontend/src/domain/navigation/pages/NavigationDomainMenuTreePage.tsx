import { useEffect, useMemo, useState, type FormEvent } from "react";
import { ArrowDown, ArrowUp, Copy, FolderTree, Plus, RefreshCcw, Trash2 } from "lucide-react";
import { useLocation, useNavigate } from "react-router";

import {
  Badge,
  Button,
  ConfirmDialog,
  Field,
  ModalDialog,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextInput,
  useToast,
} from "@/shared";
import { SelectItem } from "@/components/ui/select";
import {
  addNavigationMenuItem,
  copyNavigationMenuTree,
  deleteNavigationMenuItem,
  fetchNavigationAdminDomains,
  fetchNavigationAdminMenuTree,
  fetchNavigationAdminMenus,
  updateNavigationMenuItem,
} from "@/domain/navigation/api";
import type {
  NavigationAdminDomain,
  NavigationAdminMenu,
  NavigationAdminTreeItem,
  NavigationMenuType,
} from "@/shared/types/Navigation";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useNavigationAdminLocale, useNavigationAdminText, type NavigationAdminLocale } from "./navigationAdminText";

const ROOT_VALUE = "__ROOT__";

function flattenTree(items: NavigationAdminTreeItem[]): NavigationAdminTreeItem[] {
  return items.flatMap((item) => [item, ...flattenTree(item.children)]);
}

function collectExistingCodes(items: NavigationAdminTreeItem[]) {
  return new Set(flattenTree(items).map((item) => item.menuCode));
}

function menuTypeLabel(type: NavigationMenuType, text: ReturnType<typeof useNavigationAdminText>) {
  if (type === "Group") return text("types.group", "分组", "Group");
  if (type === "ExternalLink") return text("types.externalLink", "外部链接", "External link");
  return text("types.internal", "内部页面", "Internal page");
}

function rolesFromInput(value: string) {
  return [...new Set(value.split(",").map((role) => role.trim()).filter(Boolean))];
}

function TreeNode({
  item,
  level,
  siblingIndex,
  siblingCount,
  parentMenuCode,
  onMove,
  onRemove,
  locale,
  text,
}: {
  item: NavigationAdminTreeItem;
  level: number;
  siblingIndex: number;
  siblingCount: number;
  parentMenuCode: string | null;
  onMove: (item: NavigationAdminTreeItem, parent: string | null, position: number) => void;
  onRemove: (item: NavigationAdminTreeItem) => void;
  locale: NavigationAdminLocale;
  text: ReturnType<typeof useNavigationAdminText>;
}) {
  return (
    <div>
      <div className="group flex items-center gap-2 rounded-xl px-2 py-2 hover:bg-[var(--color-hover-surface)]" style={{ marginLeft: `${Math.min(level, 3) * 18}px` }}>
        <FolderTree className="shrink-0 text-[var(--color-on-surface-variant)]" size={16} aria-hidden="true" />
        <div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2"><span className="truncate text-sm font-medium">{item.translations[locale]}</span><Badge tone="default">{menuTypeLabel(item.menuType, text)}</Badge>{item.source === "BuiltIn" ? <Badge tone="info">{text("status.builtIn", "内置", "Built-in")}</Badge> : null}</div><div className="mt-0.5 flex flex-wrap gap-2 text-xs text-[var(--color-on-surface-variant)]"><span className="font-mono">{item.menuCode}</span>{item.visibleRoles.length ? <span>{item.visibleRoles.join(", ")}</span> : <span>{text("tree.allUsers", "所有登录用户", "All signed-in users")}</span>}</div></div>
        <div className="flex shrink-0 items-center gap-0.5 opacity-70 transition-opacity group-hover:opacity-100"><Button size="icon" variant="ghost" className="size-8" disabled={siblingIndex === 0} aria-label={text("aria.moveUp", "上移", "Move up")} onClick={() => onMove(item, parentMenuCode, siblingIndex - 1)}><ArrowUp size={14} aria-hidden="true" /></Button><Button size="icon" variant="ghost" className="size-8" disabled={siblingIndex === siblingCount - 1} aria-label={text("aria.moveDown", "下移", "Move down")} onClick={() => onMove(item, parentMenuCode, siblingIndex + 1)}><ArrowDown size={14} aria-hidden="true" /></Button><Button size="icon" variant="ghost" className="size-8" disabled={item.children.length > 0} aria-label={text("aria.remove", `移除 ${item.menuCode}`, `Remove ${item.menuCode}`)} onClick={() => onRemove(item)}><Trash2 size={14} aria-hidden="true" /></Button></div>
      </div>
      {item.children.length ? <div className="space-y-1">{item.children.map((child, index) => <TreeNode key={child.menuCode} item={child} level={level + 1} siblingIndex={index} siblingCount={item.children.length} parentMenuCode={item.menuCode} onMove={onMove} onRemove={onRemove} locale={locale} text={text} />)}</div> : null}
    </div>
  );
}

export function NavigationDomainMenuTreePage() {
  const text = useNavigationAdminText();
  const locale = useNavigationAdminLocale();
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const isApiMode = getServiceDataSource("admin") === "api";
  const requestedDomainCode = new URLSearchParams(location.search).get("domainCode");
  const [domains, setDomains] = useState<NavigationAdminDomain[]>([]);
  const [menus, setMenus] = useState<NavigationAdminMenu[]>([]);
  const [selectedDomainCode, setSelectedDomainCode] = useState(requestedDomainCode ?? "");
  const [tree, setTree] = useState<NavigationAdminTreeItem[]>([]);
  const [sourceTree, setSourceTree] = useState<NavigationAdminTreeItem[]>([]);
  const [sourceDomainCode, setSourceDomainCode] = useState("");
  const [sourceMenuCode, setSourceMenuCode] = useState(ROOT_VALUE);
  const [targetParentMenuCode, setTargetParentMenuCode] = useState(ROOT_VALUE);
  const [position, setPosition] = useState("0");
  const [newMenuCode, setNewMenuCode] = useState("");
  const [newParentMenuCode, setNewParentMenuCode] = useState(ROOT_VALUE);
  const [newVisibleRoles, setNewVisibleRoles] = useState("");
  const [copyPosition, setCopyPosition] = useState("0");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [addMenuOpen, setAddMenuOpen] = useState(false);
  const [copyMenuOpen, setCopyMenuOpen] = useState(false);
  const [itemToRemove, setItemToRemove] = useState<NavigationAdminTreeItem | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (!isApiMode) return;
    const controller = new AbortController();
    void Promise.all([
      fetchNavigationAdminDomains({ current: 1, size: 200, keyword: "" }, controller.signal),
      fetchNavigationAdminMenus({ current: 1, size: 200, keyword: "" }, controller.signal),
    ]).then(([domainPage, menuPage]) => {
      if (controller.signal.aborted) return;
      setDomains(domainPage.records);
      setMenus(menuPage.records);
      const nextDomain = requestedDomainCode && domainPage.records.some((domain) => domain.domainCode === requestedDomainCode)
        ? requestedDomainCode
        : domainPage.records[0]?.domainCode ?? "";
      setSelectedDomainCode(nextDomain);
      setSourceDomainCode((current) => current || domainPage.records[0]?.domainCode || "");
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadTreeData", "菜单编排数据加载失败", "Menu arrangement data could not be loaded"));
    });
    return () => controller.abort();
  }, [isApiMode, refreshKey, requestedDomainCode, text, toast]);

  useEffect(() => {
    if (!isApiMode || !selectedDomainCode) {
      setTree([]);
      return;
    }
    const controller = new AbortController();
    setLoading(true);
    void fetchNavigationAdminMenuTree(selectedDomainCode, controller.signal).then((items) => {
      if (!controller.signal.aborted) setTree(items);
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadTree", "菜单树加载失败", "Menu tree could not be loaded"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [isApiMode, selectedDomainCode, text, toast]);

  useEffect(() => {
    if (!isApiMode || !sourceDomainCode) {
      setSourceTree([]);
      return;
    }
    const controller = new AbortController();
    void fetchNavigationAdminMenuTree(sourceDomainCode, controller.signal).then((items) => {
      if (!controller.signal.aborted) {
        setSourceTree(items);
        setSourceMenuCode(ROOT_VALUE);
      }
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadSourceTree", "源菜单树加载失败", "Source menu tree could not be loaded"));
    });
    return () => controller.abort();
  }, [isApiMode, sourceDomainCode, text, toast]);

  const existingCodes = useMemo(() => collectExistingCodes(tree), [tree]);
  const availableMenus = useMemo(() => menus.filter((menu) => !existingCodes.has(menu.menuCode)), [existingCodes, menus]);
  const groupOptions = useMemo(() => flattenTree(tree).filter((item) => item.menuType === "Group"), [tree]);
  const sourceOptions = useMemo(() => flattenTree(sourceTree), [sourceTree]);

  function changeDomain(domainCode: string) {
    setSelectedDomainCode(domainCode);
    navigate(`/admin/navigation/menu-tree?domainCode=${encodeURIComponent(domainCode)}`);
  }

  function openAddMenu() {
    setNewMenuCode("");
    setNewParentMenuCode(ROOT_VALUE);
    setPosition("0");
    setNewVisibleRoles("");
    setAddMenuOpen(true);
  }

  function openCopyMenu() {
    setSourceMenuCode(ROOT_VALUE);
    setTargetParentMenuCode(ROOT_VALUE);
    setCopyPosition("0");
    setCopyMenuOpen(true);
  }

  async function addMenu(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedDomainCode || !newMenuCode) return;
    setSaving(true);
    try {
      const nextTree = await addNavigationMenuItem(selectedDomainCode, { menuCode: newMenuCode, parentMenuCode: newParentMenuCode === ROOT_VALUE ? null : newParentMenuCode, position: Number(position) || 0, visibleRoles: rolesFromInput(newVisibleRoles) });
      setTree(nextTree);
      setNewMenuCode("");
      setNewVisibleRoles("");
      setAddMenuOpen(false);
      toast.success(text("messages.itemAdded", "菜单已加入领域", "Menu added to domain"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.addItem", "菜单加入失败", "Menu could not be added"));
    } finally {
      setSaving(false);
    }
  }

  async function moveMenu(item: NavigationAdminTreeItem, parent: string | null, nextPosition: number) {
    if (!selectedDomainCode) return;
    try {
      const nextTree = await updateNavigationMenuItem(selectedDomainCode, item.menuCode, { parentMenuCode: parent, position: nextPosition, visibleRoles: item.visibleRoles });
      setTree(nextTree);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.moveItem", "菜单移动失败", "Menu could not be moved"));
    }
  }

  async function confirmRemoveMenu() {
    if (!selectedDomainCode || !itemToRemove) return;
    setSaving(true);
    try {
      const nextTree = await deleteNavigationMenuItem(selectedDomainCode, itemToRemove.menuCode);
      setTree(nextTree);
      toast.success(text("messages.itemRemoved", "菜单已从领域移除", "Menu removed from domain"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.removeItem", "菜单移除失败", "Menu could not be removed"));
    } finally {
      setSaving(false);
      setItemToRemove(null);
    }
  }

  async function copyMenuTree(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedDomainCode || !sourceDomainCode) return;
    setSaving(true);
    try {
      const nextTree = await copyNavigationMenuTree(selectedDomainCode, { sourceDomainCode, sourceMenuCode: sourceMenuCode === ROOT_VALUE ? null : sourceMenuCode, targetParentMenuCode: targetParentMenuCode === ROOT_VALUE ? null : targetParentMenuCode, position: Number(copyPosition) || 0 });
      setTree(nextTree);
      setCopyMenuOpen(false);
      toast.success(text("messages.treeCopied", "菜单树复制成功", "Menu tree copied"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.copyTree", "菜单树复制失败", "Menu tree could not be copied"));
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <PageHeader title={text("tree.title", "菜单编排", "Menu arrangement")} actions={<><Button variant="secondary" disabled={loading} onClick={() => setRefreshKey((value) => value + 1)}><RefreshCcw size={16} aria-hidden="true" />{text("actions.refresh", "刷新", "Refresh")}</Button><Button variant="secondary" disabled={!selectedDomainCode || saving} onClick={openAddMenu}><Plus size={16} aria-hidden="true" />{text("tree.addTitle", "加入菜单", "Add menu")}</Button><Button disabled={!selectedDomainCode || saving} onClick={openCopyMenu}><Copy size={16} aria-hidden="true" />{text("tree.copyTitle", "跨领域复制", "Copy across domains")}</Button></>} />
      <div className="mb-5 rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-5"><SelectInput className="min-w-64" aria-label={text("tree.domain", "领域", "Domain")} value={selectedDomainCode} onValueChange={(value) => { if (value) changeDomain(value); }}>{domains.map((domain) => <SelectItem key={domain.domainCode} value={domain.domainCode}>{domain.name[locale]}</SelectItem>)}</SelectInput></div>
      <SurfaceCard>
        <div className="mb-4 flex items-start justify-between gap-3"><h2 className="text-base font-semibold">{text("tree.currentTitle", "当前领域菜单树", "Current domain menu tree")}</h2><Badge tone="info">{text("tree.maxDepth", "最多 3 级", "Up to 3 levels")}</Badge></div>
        {loading ? <p className="py-8 text-center text-sm text-[var(--color-on-surface-variant)]">{text("loadingTree", "正在加载菜单树…", "Loading menu tree…")}</p> : tree.length ? <><div className="mb-1 flex justify-end px-2 text-xs font-medium text-[var(--color-on-surface-variant)]">{text("columns.actions", "操作", "Actions")}</div><div className="space-y-1">{tree.map((item, index) => <TreeNode key={item.menuCode} item={item} level={0} siblingIndex={index} siblingCount={tree.length} parentMenuCode={null} onMove={moveMenu} onRemove={setItemToRemove} locale={locale} text={text} />)}</div></> : <p className="py-8 text-center text-sm text-[var(--color-on-surface-variant)]">{text("tree.empty", "当前领域还没有菜单。", "This domain has no menus yet.")}</p>}
      </SurfaceCard>

      <ModalDialog open={addMenuOpen} title={text("tree.addTitle", "加入菜单", "Add menu")} className="max-h-[88vh] max-w-xl overflow-y-auto">
          <form className="space-y-4" onSubmit={(event) => void addMenu(event)}>
            <Field label={text("fields.menu", "菜单", "Menu")} required><SelectInput value={newMenuCode} placeholder={text("tree.selectMenu", "选择菜单定义", "Select a menu definition")} onValueChange={(value) => { if (value) setNewMenuCode(value); }}>{availableMenus.length ? availableMenus.map((menu) => <SelectItem key={menu.menuCode} value={menu.menuCode}>{menu.translations[locale]} · {menu.menuCode}</SelectItem>) : <SelectItem disabled value="__NO_AVAILABLE_MENU__">{text("tree.noAvailableMenus", "暂无可加入的菜单", "No menus available")}</SelectItem>}</SelectInput></Field>
            <Field label={text("fields.parent", "父级分组", "Parent group")}><SelectInput value={newParentMenuCode} onValueChange={(value) => { if (value) setNewParentMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.root", "根节点", "Root")}</SelectItem>{groupOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
            <div className="grid grid-cols-2 gap-3"><Field label={text("fields.position", "位置", "Position")}><TextInput type="number" min="0" value={position} onChange={(event) => setPosition(event.target.value)} /></Field><Field label={text("fields.visibleRoles", "可见角色", "Visible roles")}><TextInput value={newVisibleRoles} placeholder="ADMIN" onChange={(event) => setNewVisibleRoles(event.target.value)} /></Field></div>
            <div className="flex justify-end gap-2"><Button type="button" variant="ghost" disabled={saving} onClick={() => setAddMenuOpen(false)}>{text("actions.cancel", "取消", "Cancel")}</Button><Button type="submit" disabled={!newMenuCode || saving} aria-busy={saving}><Plus size={15} aria-hidden="true" />{text("actions.add", "加入", "Add")}</Button></div>
          </form>
      </ModalDialog>

      <ModalDialog open={copyMenuOpen} title={text("tree.copyTitle", "跨领域复制", "Copy across domains")} className="max-h-[88vh] max-w-xl overflow-y-auto">
          <form className="space-y-4" onSubmit={(event) => void copyMenuTree(event)}>
            <Field label={text("fields.sourceDomain", "源领域", "Source domain")} required><SelectInput value={sourceDomainCode} onValueChange={(value) => { if (value) setSourceDomainCode(value); }}>{domains.map((domain) => <SelectItem key={domain.domainCode} value={domain.domainCode}>{domain.name[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.sourceMenu", "源菜单", "Source menu")} hint={text("tree.copyHint", "目标领域存在同编码菜单时，整个复制操作会失败并回滚。", "If any menu code already exists in the target domain, the whole copy is rejected and rolled back.")}><SelectInput value={sourceMenuCode} onValueChange={(value) => { if (value) setSourceMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.wholeTree", "完整菜单树", "Entire menu tree")}</SelectItem>{sourceOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]} · {item.menuCode}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.targetParent", "目标父级分组", "Target parent group")}><SelectInput value={targetParentMenuCode} onValueChange={(value) => { if (value) setTargetParentMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.root", "根节点", "Root")}</SelectItem>{groupOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.position", "位置", "Position")}><TextInput type="number" min="0" value={copyPosition} onChange={(event) => setCopyPosition(event.target.value)} /></Field>
            <div className="flex justify-end gap-2"><Button type="button" variant="ghost" disabled={saving} onClick={() => setCopyMenuOpen(false)}>{text("actions.cancel", "取消", "Cancel")}</Button><Button type="submit" disabled={!sourceDomainCode || saving} aria-busy={saving}><Copy size={15} aria-hidden="true" />{text("actions.copy", "复制", "Copy")}</Button></div>
          </form>
      </ModalDialog>
      <ConfirmDialog open={Boolean(itemToRemove)} title={text("tree.removeTitle", "从领域移除菜单", "Remove menu from domain")} description={text("tree.removeDescription", `确定移除 ${itemToRemove?.menuCode ?? ""}？这只会删除当前领域的编排关系，不会删除菜单池定义。`, `Remove ${itemToRemove?.menuCode ?? ""}? This deletes only the arrangement relation; the menu pool definition remains.`)} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.remove", "移除", "Remove")} pending={saving} onCancel={() => setItemToRemove(null)} onConfirm={() => void confirmRemoveMenu()} />
    </>
  );
}
