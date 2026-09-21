import { useEffect, useMemo, useState, type FormEvent } from "react";
import { ArrowDown, ArrowUp, Copy, FolderTree, PanelRightOpen, Plus, RefreshCcw, RotateCcw, Trash2 } from "lucide-react";
import { useLocation, useNavigate } from "react-router";

import { Field, SelectInput, TextInput } from "@/components/forms";
import { ModalDialog } from "@/components/modal-dialog";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SelectItem } from "@/components/ui/select";
import {
  addNavigationMenuItem,
  copyNavigationMenuTree,
  deleteNavigationMenuItem,
  fetchNavigationAdminMenuTree,
  fetchNavigationAdminMenuTrees,
  fetchNavigationAdminMenus,
  restoreNavigationDefaultMenuTrees,
  updateNavigationMenuItem,
} from "@/domain/navigation/api";
import type {
  NavigationAdminMenu,
  NavigationAdminMenuTree,
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
  if (type === "Widget") return text("types.widget", "面板组件", "Widget");
  return text("types.internal", "内部页面", "Internal page");
}

function TreeNode({
  item,
  level,
  siblingIndex,
  siblingCount,
  parentMenuCode,
  onMove,
  onChildDisplayChange,
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
  onChildDisplayChange: (item: NavigationAdminTreeItem) => void;
  onRemove: (item: NavigationAdminTreeItem) => void;
  locale: NavigationAdminLocale;
  text: ReturnType<typeof useNavigationAdminText>;
}) {
  return (
    <div>
      <div className="group flex items-center gap-2 rounded-xl px-2 py-2 hover:bg-[var(--color-hover-surface)]" style={{ marginLeft: `${Math.min(level, 3) * 18}px` }}>
        <FolderTree className="shrink-0 text-[var(--color-on-surface-variant)]" size={16} aria-hidden="true" />
        <div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2"><span className="truncate text-sm font-medium">{item.translations[locale]}</span><Badge variant="default">{menuTypeLabel(item.menuType, text)}</Badge>{item.menuType === "Group" ? <Badge variant={item.childDisplay === "Flyout" ? "info" : "default"}>{item.childDisplay === "Flyout" ? text("tree.flyout", "右侧展开", "Right-side flyout") : text("tree.inline", "侧边展开", "Inline")}</Badge> : null}{item.source === "BuiltIn" ? <Badge variant="info">{text("status.builtIn", "内置", "Built-in")}</Badge> : null}</div><div className="mt-0.5 flex flex-wrap gap-2 text-xs text-[var(--color-on-surface-variant)]"><span className="font-mono">{item.menuCode}</span></div></div>
        <div className="flex shrink-0 items-center gap-0.5 opacity-70 transition-opacity group-hover:opacity-100">{item.menuType === "Group" ? <Button size="icon" variant="ghost" className="size-8" aria-label={item.childDisplay === "Flyout" ? text("aria.setInline", "改为侧边展开", "Use inline expansion") : text("aria.setFlyout", "改为右侧展开", "Use right-side flyout")} title={item.childDisplay === "Flyout" ? text("tree.inline", "侧边展开", "Inline") : text("tree.flyout", "右侧展开", "Right-side flyout")} onClick={() => onChildDisplayChange(item)}><PanelRightOpen size={14} aria-hidden="true" /></Button> : null}
        <Button size="icon" variant="ghost" className="size-8" disabled={siblingIndex === 0} aria-label={text("aria.moveUp", "上移", "Move up")} onClick={() => onMove(item, parentMenuCode, siblingIndex - 1)}><ArrowUp size={14} aria-hidden="true" /></Button><Button size="icon" variant="ghost" className="size-8" disabled={siblingIndex === siblingCount - 1} aria-label={text("aria.moveDown", "下移", "Move down")} onClick={() => onMove(item, parentMenuCode, siblingIndex + 1)}><ArrowDown size={14} aria-hidden="true" /></Button><Button size="icon" variant="ghost" className="size-8" disabled={item.children.length > 0} aria-label={text("aria.remove", `移除 ${item.menuCode}`, `Remove ${item.menuCode}`)} onClick={() => onRemove(item)}><Trash2 size={14} aria-hidden="true" /></Button></div>
      </div>
      {item.children.length ? <div className="space-y-1">{item.children.map((child, index) => <TreeNode key={child.menuCode} item={child} level={level + 1} siblingIndex={index} siblingCount={item.children.length} parentMenuCode={item.menuCode} onMove={onMove} onChildDisplayChange={onChildDisplayChange} onRemove={onRemove} locale={locale} text={text} />)}</div> : null}
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
  const requestedTreeCode = new URLSearchParams(location.search).get("treeCode")
    ?? new URLSearchParams(location.search).get("domainCode");
  const [menuTrees, setMenuTrees] = useState<NavigationAdminMenuTree[]>([]);
  const [menus, setMenus] = useState<NavigationAdminMenu[]>([]);
  const [selectedTreeCode, setSelectedTreeCode] = useState(requestedTreeCode ?? "");
  const [tree, setTree] = useState<NavigationAdminTreeItem[]>([]);
  const [sourceTree, setSourceTree] = useState<NavigationAdminTreeItem[]>([]);
  const [sourceTreeCode, setSourceTreeCode] = useState("");
  const [sourceMenuCode, setSourceMenuCode] = useState(ROOT_VALUE);
  const [targetParentMenuCode, setTargetParentMenuCode] = useState(ROOT_VALUE);
  const [position, setPosition] = useState("0");
  const [newMenuCode, setNewMenuCode] = useState("");
  const [newParentMenuCode, setNewParentMenuCode] = useState(ROOT_VALUE);
  const [copyPosition, setCopyPosition] = useState("0");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [addMenuOpen, setAddMenuOpen] = useState(false);
  const [copyMenuOpen, setCopyMenuOpen] = useState(false);
  const [restoreDefaultTreesOpen, setRestoreDefaultTreesOpen] = useState(false);
  const [itemToRemove, setItemToRemove] = useState<NavigationAdminTreeItem | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (!isApiMode) return;
    const controller = new AbortController();
    void Promise.all([
      fetchNavigationAdminMenuTrees(controller.signal),
      fetchNavigationAdminMenus({ current: 1, size: 200, keyword: "" }, controller.signal),
    ]).then(([trees, menuPage]) => {
      if (controller.signal.aborted) return;
      setMenuTrees(trees);
      setMenus(menuPage.records);
      const nextTree = requestedTreeCode && trees.some((tree) => tree.treeCode === requestedTreeCode)
        ? requestedTreeCode
        : trees[0]?.treeCode ?? "";
      setSelectedTreeCode(nextTree);
      setSourceTreeCode((current) => current || trees[0]?.treeCode || "");
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadTreeData", "菜单编排数据加载失败", "Menu arrangement data could not be loaded"));
    });
    return () => controller.abort();
  }, [isApiMode, refreshKey, requestedTreeCode, text, toast]);

  useEffect(() => {
    if (!isApiMode || !selectedTreeCode) {
      setTree([]);
      return;
    }
    const controller = new AbortController();
    setLoading(true);
    void fetchNavigationAdminMenuTree(selectedTreeCode, controller.signal).then((items) => {
      if (!controller.signal.aborted) setTree(items);
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadTree", "菜单树加载失败", "Menu tree could not be loaded"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [isApiMode, refreshKey, selectedTreeCode, text, toast]);

  useEffect(() => {
    if (!isApiMode || !sourceTreeCode) {
      setSourceTree([]);
      return;
    }
    const controller = new AbortController();
    void fetchNavigationAdminMenuTree(sourceTreeCode, controller.signal).then((items) => {
      if (!controller.signal.aborted) {
        setSourceTree(items);
        setSourceMenuCode(ROOT_VALUE);
      }
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : text("errors.loadSourceTree", "源菜单树加载失败", "Source menu tree could not be loaded"));
    });
    return () => controller.abort();
  }, [isApiMode, refreshKey, sourceTreeCode, text, toast]);

  const existingCodes = useMemo(() => collectExistingCodes(tree), [tree]);
  const availableMenus = useMemo(() => menus.filter((menu) => !existingCodes.has(menu.menuCode)), [existingCodes, menus]);
  const groupOptions = useMemo(() => flattenTree(tree).filter((item) => item.menuType === "Group"), [tree]);
  const sourceOptions = useMemo(() => flattenTree(sourceTree), [sourceTree]);

  function changeTree(treeCode: string) {
    setSelectedTreeCode(treeCode);
    navigate(`/admin/navigation/menu-tree?treeCode=${encodeURIComponent(treeCode)}`);
  }

  function openAddMenu() {
    setNewMenuCode("");
    setNewParentMenuCode(ROOT_VALUE);
    setPosition("0");
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
    if (!selectedTreeCode || !newMenuCode) return;
    setSaving(true);
    try {
      const nextTree = await addNavigationMenuItem(selectedTreeCode, { menuCode: newMenuCode, parentMenuCode: newParentMenuCode === ROOT_VALUE ? null : newParentMenuCode, position: Number(position) || 0 });
      setTree(nextTree);
      setNewMenuCode("");
      setAddMenuOpen(false);
      toast.success(text("messages.itemAdded", "菜单已加入编排", "Menu added to arrangement"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.addItem", "菜单加入失败", "Menu could not be added"));
    } finally {
      setSaving(false);
    }
  }

  async function moveMenu(item: NavigationAdminTreeItem, parent: string | null, nextPosition: number) {
    if (!selectedTreeCode) return;
    try {
      const nextTree = await updateNavigationMenuItem(selectedTreeCode, item.menuCode, { parentMenuCode: parent, position: nextPosition, childDisplay: item.childDisplay });
      setTree(nextTree);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.moveItem", "菜单移动失败", "Menu could not be moved"));
    }
  }

  async function toggleChildDisplay(item: NavigationAdminTreeItem) {
    if (!selectedTreeCode || item.menuType !== "Group") return;
    try {
      const childDisplay = item.childDisplay === "Flyout" ? "Inline" : "Flyout";
      const nextTree = await updateNavigationMenuItem(selectedTreeCode, item.menuCode, {
        parentMenuCode: item.parentMenuCode ?? null,
        position: item.position,
        childDisplay,
      });
      setTree(nextTree);
      toast.success(childDisplay === "Flyout"
        ? text("messages.flyoutEnabled", "已改为右侧展开", "Right-side flyout enabled")
        : text("messages.inlineEnabled", "已改为侧边展开", "Inline expansion enabled"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.updateDisplay", "子菜单展示方式更新失败", "Submenu display could not be updated"));
    }
  }

  async function confirmRemoveMenu() {
    if (!selectedTreeCode || !itemToRemove) return;
    setSaving(true);
    try {
      const nextTree = await deleteNavigationMenuItem(selectedTreeCode, itemToRemove.menuCode);
      setTree(nextTree);
      toast.success(text("messages.itemRemoved", "菜单已从编排中移除", "Menu removed from arrangement"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.removeItem", "菜单移除失败", "Menu could not be removed"));
    } finally {
      setSaving(false);
      setItemToRemove(null);
    }
  }

  async function copyMenuTree(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedTreeCode || !sourceTreeCode) return;
    setSaving(true);
    try {
      const nextTree = await copyNavigationMenuTree(selectedTreeCode, { sourceTreeCode, sourceMenuCode: sourceMenuCode === ROOT_VALUE ? null : sourceMenuCode, targetParentMenuCode: targetParentMenuCode === ROOT_VALUE ? null : targetParentMenuCode, position: Number(copyPosition) || 0 });
      setTree(nextTree);
      setCopyMenuOpen(false);
      toast.success(text("messages.treeCopied", "菜单树复制成功", "Menu tree copied"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.copyTree", "菜单树复制失败", "Menu tree could not be copied"));
    } finally {
      setSaving(false);
    }
  }

  async function confirmRestoreDefaultMenuTrees() {
    setSaving(true);
    try {
      const result = await restoreNavigationDefaultMenuTrees();
      setRestoreDefaultTreesOpen(false);
      setRefreshKey((value) => value + 1);
      toast.success(text(
        "messages.defaultTreesRestored",
        `已生成 ${result.restoredItemCount} 个默认菜单节点`,
        `${result.restoredItemCount} default menu nodes generated`,
      ));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.restoreDefaultTrees", "默认菜单树生成失败", "Default menu trees could not be generated"));
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <PageHeader title={text("tree.title", "菜单编排", "Menu arrangement")} actions={<><Button variant="secondary" disabled={loading} onClick={() => setRefreshKey((value) => value + 1)}><RefreshCcw size={16} aria-hidden="true" />{text("actions.refresh", "刷新", "Refresh")}</Button><Button variant="secondary" disabled={!isApiMode || saving} onClick={() => setRestoreDefaultTreesOpen(true)}><RotateCcw size={16} aria-hidden="true" />{text("tree.restoreDefaultTrees", "生成所有默认菜单树", "Generate all default menu trees")}</Button><Button variant="secondary" disabled={!selectedTreeCode || saving} onClick={openAddMenu}><Plus size={16} aria-hidden="true" />{text("tree.addTitle", "加入菜单", "Add menu")}</Button><Button disabled={!selectedTreeCode || saving} onClick={openCopyMenu}><Copy size={16} aria-hidden="true" />{text("tree.copyTitle", "跨菜单树复制", "Copy across menu trees")}</Button></>} />
      <SurfaceCard>
        <div className="mb-4 border-[var(--color-card-border)]"><SelectInput className="min-w-64" aria-label={text("tree.target", "编排位置", "Arrangement target")} value={selectedTreeCode} onValueChange={(value) => { if (value) changeTree(value); }}>{menuTrees.map((menuTree) => <SelectItem key={menuTree.treeCode} value={menuTree.treeCode}>{menuTree.name[locale]} · {menuTree.treeKind === "Panel" ? text("tree.panel", "面板", "Panel") : text("tree.navigation", "导航", "Navigation")}</SelectItem>)}</SelectInput></div>
        <div className="mb-4 flex items-start justify-between gap-3"><h2 className="text-base font-semibold">{text("tree.currentTitle", "当前编排菜单树", "Current arranged menu tree")}</h2><Badge variant="info">{text("tree.maxDepth", "最多 3 级", "Up to 3 levels")}</Badge></div>
        {loading ? <p className="py-8 text-center text-sm text-[var(--color-on-surface-variant)]">{text("loadingTree", "正在加载菜单树…", "Loading menu tree…")}</p> : tree.length ? <><div className="mb-1 flex justify-end px-2 text-xs font-medium text-[var(--color-on-surface-variant)]">{text("columns.actions", "操作", "Actions")}</div><div className="space-y-1">{tree.map((item, index) => <TreeNode key={item.menuCode} item={item} level={0} siblingIndex={index} siblingCount={tree.length} parentMenuCode={null} onMove={moveMenu} onChildDisplayChange={toggleChildDisplay} onRemove={setItemToRemove} locale={locale} text={text} />)}</div></> : <p className="py-8 text-center text-sm text-[var(--color-on-surface-variant)]">{text("tree.empty", "当前编排位置还没有菜单。", "This arrangement target has no menus yet.")}</p>}
      </SurfaceCard>

      <ModalDialog open={addMenuOpen} title={text("tree.addTitle", "加入菜单", "Add menu")} className="max-h-[88vh] max-w-xl overflow-y-auto">
          <form className="space-y-4" onSubmit={(event) => void addMenu(event)}>
            <Field label={text("fields.menu", "菜单", "Menu")} required><SelectInput value={newMenuCode} placeholder={text("tree.selectMenu", "选择菜单定义", "Select a menu definition")} onValueChange={(value) => { if (value) setNewMenuCode(value); }}>{availableMenus.length ? availableMenus.map((menu) => <SelectItem key={menu.menuCode} value={menu.menuCode}>{menu.translations[locale]} · {menu.menuCode}</SelectItem>) : <SelectItem disabled value="__NO_AVAILABLE_MENU__">{text("tree.noAvailableMenus", "暂无可加入的菜单", "No menus available")}</SelectItem>}</SelectInput></Field>
            <Field label={text("fields.parent", "父级分组", "Parent group")}><SelectInput value={newParentMenuCode} onValueChange={(value) => { if (value) setNewParentMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.root", "根节点", "Root")}</SelectItem>{groupOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.position", "位置", "Position")}><TextInput type="number" min="0" value={position} onChange={(event) => setPosition(event.target.value)} /></Field>
            <div className="flex justify-end gap-2"><Button type="button" variant="ghost" disabled={saving} onClick={() => setAddMenuOpen(false)}>{text("actions.cancel", "取消", "Cancel")}</Button><Button type="submit" disabled={!newMenuCode || saving} aria-busy={saving}><Plus size={15} aria-hidden="true" />{text("actions.add", "加入", "Add")}</Button></div>
          </form>
      </ModalDialog>

      <ModalDialog open={copyMenuOpen} title={text("tree.copyTitle", "跨菜单树复制", "Copy across menu trees")} className="max-h-[88vh] max-w-xl overflow-y-auto">
          <form className="space-y-4" onSubmit={(event) => void copyMenuTree(event)}>
            <Field label={text("fields.sourceTree", "源菜单树", "Source menu tree")} required><SelectInput value={sourceTreeCode} onValueChange={(value) => { if (value) setSourceTreeCode(value); }}>{menuTrees.map((menuTree) => <SelectItem key={menuTree.treeCode} value={menuTree.treeCode}>{menuTree.name[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.sourceMenu", "源菜单", "Source menu")} hint={text("tree.copyHint", "目标菜单树存在同编码菜单时，整个复制操作会失败并回滚。", "If any menu code already exists in the target menu tree, the whole copy is rejected and rolled back.")}><SelectInput value={sourceMenuCode} onValueChange={(value) => { if (value) setSourceMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.wholeTree", "完整菜单树", "Entire menu tree")}</SelectItem>{sourceOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]} · {item.menuCode}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.targetParent", "目标父级分组", "Target parent group")}><SelectInput value={targetParentMenuCode} onValueChange={(value) => { if (value) setTargetParentMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.root", "根节点", "Root")}</SelectItem>{groupOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.position", "位置", "Position")}><TextInput type="number" min="0" value={copyPosition} onChange={(event) => setCopyPosition(event.target.value)} /></Field>
            <div className="flex justify-end gap-2"><Button type="button" variant="ghost" disabled={saving} onClick={() => setCopyMenuOpen(false)}>{text("actions.cancel", "取消", "Cancel")}</Button><Button type="submit" disabled={!sourceTreeCode || saving} aria-busy={saving}><Copy size={15} aria-hidden="true" />{text("actions.copy", "复制", "Copy")}</Button></div>
          </form>
      </ModalDialog>
      <ConfirmDialog open={restoreDefaultTreesOpen} title={text("tree.restoreDefaultTrees", "生成所有默认菜单树", "Generate all default menu trees")} description={text("tree.restoreDefaultTreesDescription", "会补齐所有缺失的内置默认菜单节点；已有菜单及自定义编排不会被删除或覆盖。", "This adds every missing built-in default menu node. Existing menus and custom arrangements are not deleted or overwritten.")} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.generate", "生成", "Generate")} pending={saving} onCancel={() => setRestoreDefaultTreesOpen(false)} onConfirm={() => void confirmRestoreDefaultMenuTrees()} />
      <ConfirmDialog open={Boolean(itemToRemove)} title={text("tree.removeTitle", "从编排移除菜单", "Remove menu from arrangement")} description={text("tree.removeDescription", `确定移除 ${itemToRemove?.menuCode ?? ""}？这只会删除当前菜单树的编排关系，不会删除菜单定义。`, `Remove ${itemToRemove?.menuCode ?? ""}? This deletes only the current menu-tree arrangement; the menu definition remains.`)} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.remove", "移除", "Remove")} pending={saving} onCancel={() => setItemToRemove(null)} onConfirm={() => void confirmRemoveMenu()} />
    </>
  );
}
