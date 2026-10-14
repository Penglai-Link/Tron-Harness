import { GripVertical, Trash2, FolderTree, Settings2 } from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent } from "react";
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
import { InfoTooltip } from "@/components/info-tooltip";
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
import { getNavigationIcon } from "@/shared/lib/navigationMenuMapper";
import { menuDropPlacement } from "./menuDropPlacement";
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

function TreeNode({ item, level, siblingIndex, siblingCount, onMove, onSelect, selectedCode, busy, locale, text, draggedCode, onDragChange, treeItems }: {
  treeItems: NavigationAdminTreeItem[];
  draggedCode: string | null;
  onDragChange: (code: string | null) => void;
  item: NavigationAdminTreeItem;
  level: number;
  siblingIndex: number;
  siblingCount: number;
  onMove: (item: NavigationAdminTreeItem, parent: string | null, position: number) => void;
  onSelect: (code: string) => void;
  selectedCode: string;
  busy: boolean;
  locale: NavigationAdminLocale;
  text: ReturnType<typeof useNavigationAdminText>;
}) {
  const Icon = getNavigationIcon(item.icon) ?? FolderTree;
  const [dropEdge, setDropEdge] = useState<"before" | "after" | "inside" | null>(null);
  const activeDropEdge = draggedCode ? dropEdge : null;
  const parentName = item.parentMenuCode
    ? flattenTree(treeItems).find((node) => node.menuCode === item.parentMenuCode)?.translations[locale]
    : locale === "zh-CN" ? "顶层" : "Top level";
  const placementLabel = locale === "zh-CN"
    ? `${parentName} · 放在「${item.translations[locale]}」${activeDropEdge === "before" ? "之前" : "之后"}`
    : `${parentName} · ${activeDropEdge === "before" ? "Before" : "After"} ${item.translations[locale]}`;
  return (
    <li className="list-none">
      <div
        draggable={!busy}
        onDragStart={(event) => {
          event.stopPropagation();
          event.dataTransfer.effectAllowed = "move";
          event.dataTransfer.setData("application/x-tron-menu", JSON.stringify({ code: item.menuCode, parent: item.parentMenuCode, index: siblingIndex }));
          onDragChange(item.menuCode);
          onSelect(item.menuCode);
        }}
        onDragEnd={() => { onDragChange(null); setDropEdge(null); }}
        onDragOver={(event) => {
          if (busy || !draggedCode) return;
          const rect = event.currentTarget.getBoundingClientRect();
          const placement = menuDropPlacement(treeItems, draggedCode, item.menuCode, (event.clientY - rect.top) / rect.height);
          if (!placement) { setDropEdge(null); return; }
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
          setDropEdge(placement.edge);
        }}
        onDragLeave={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDropEdge(null);
        }}
        onDrop={(event) => {
          event.preventDefault();
          setDropEdge(null);
          onDragChange(null);
          if (busy || !draggedCode) return;
          const rect = event.currentTarget.getBoundingClientRect();
          const placement = menuDropPlacement(treeItems, draggedCode, item.menuCode, (event.clientY - rect.top) / rect.height);
          if (placement) onMove(placement.item, placement.parent, placement.position);
        }}
        data-menu-code={item.menuCode} className={`group relative flex items-center gap-1 rounded-lg pr-1 transition-colors ${selectedCode === item.menuCode ? "bg-[var(--color-hover-surface)]" : "hover:bg-[var(--color-hover-surface)]"}`} style={{ marginLeft: `${level * 18}px`, opacity: draggedCode === item.menuCode ? 0.4 : undefined, boxShadow: activeDropEdge === "inside" ? "inset 0 0 0 2px var(--color-primary), 0 0 16px color-mix(in srgb, var(--color-primary) 18%, transparent)" : undefined, backgroundColor: activeDropEdge === "inside" ? "color-mix(in srgb, var(--color-primary) 16%, var(--color-surface))" : undefined }}>
        {activeDropEdge && activeDropEdge !== "inside" && (
          <div aria-hidden="true" className={`pointer-events-none absolute inset-x-2 z-20 h-[3px] rounded-full bg-[var(--color-primary)] shadow-[0_0_10px_var(--color-primary)] ${activeDropEdge === "before" ? "-top-px" : "-bottom-px"}`}>
            <span className="absolute -left-1 -top-[3px] size-[9px] rounded-full border-2 border-[var(--color-primary)] bg-[var(--color-surface)]" />
            <span className="absolute right-3 top-1/2 max-w-[85%] -translate-y-1/2 truncate rounded-md border border-[var(--color-primary)] bg-[var(--color-surface)] px-2 py-1 text-xs font-semibold text-[var(--color-primary)] shadow-sm">{placementLabel}</span>
          </div>
        )}
        <button type="button" disabled={busy} aria-pressed={selectedCode === item.menuCode} onClick={() => onSelect(item.menuCode)} className="flex min-w-0 flex-1 items-center gap-3 rounded-lg px-3 py-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]">
          <Icon size={17} className="shrink-0 text-[var(--color-on-surface-variant)]" aria-hidden="true" />
          <span className="min-w-0 flex-1 truncate text-sm font-medium">{item.translations[locale]}</span>
          {activeDropEdge === "inside" && <span className="shrink-0 rounded-md bg-[var(--color-primary)] px-2 py-1 text-xs font-semibold text-[var(--color-on-primary)]">{locale === "zh-CN" ? "移入此分组" : "Move into group"}</span>}
          <span className="shrink-0 text-xs text-[var(--color-on-surface-variant)]">{menuTypeLabel(item.menuType, text)}</span>
        </button>
        <button
          type="button"
          disabled={busy}
          draggable={!busy}
          aria-label={locale === "zh-CN" ? `拖动排序 ${item.translations[locale]}` : `Drag to reorder ${item.translations[locale]}`}
          title={locale === "zh-CN" ? "拖动排序；键盘使用 Alt + 上下方向键" : "Drag to reorder; use Alt + arrow keys with a keyboard"}
          className="cursor-grab select-none px-2 py-3 text-[var(--color-on-surface-variant)] active:cursor-grabbing"
          onKeyDown={(event) => {
            if (!event.altKey || !["ArrowUp", "ArrowDown"].includes(event.key)) return;
            event.preventDefault();
            const position = siblingIndex + (event.key === "ArrowUp" ? -1 : 1);
            if (position >= 0 && position < siblingCount) onMove(item, item.parentMenuCode, position);
          }}
        ><GripVertical size={16} className="pointer-events-none" aria-hidden="true" /></button>
      </div>
      {item.children.length > 0 ? <ul>{item.children.map((child, index) => <TreeNode key={child.menuCode} item={child} level={level + 1} siblingIndex={index} siblingCount={item.children.length} treeItems={treeItems} draggedCode={draggedCode} onDragChange={onDragChange} onMove={onMove} onSelect={onSelect} selectedCode={selectedCode} busy={busy} locale={locale} text={text} />)}</ul> : null}
    </li>
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
  const [selectedMenuCode, setSelectedMenuCode] = useState("");
  const [editParent, setEditParent] = useState(ROOT_VALUE);
  const [editDisplay, setEditDisplay] = useState<"Inline" | "Flyout">("Inline");
  const [editPosition, setEditPosition] = useState("0");
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
  const [draggedCode, setDraggedCode] = useState<string | null>(null);
  const [rootDropActive, setRootDropActive] = useState(false);

  useEffect(() => { if (!draggedCode) setRootDropActive(false); }, [draggedCode]);
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
      setLoading(false);
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
    if (!isApiMode || !sourceTreeCode || !copyMenuOpen) {
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
  }, [copyMenuOpen, isApiMode, refreshKey, sourceTreeCode, text, toast]);

  const existingCodes = useMemo(() => collectExistingCodes(tree), [tree]);
  const availableMenus = useMemo(() => menus.filter((menu) => !existingCodes.has(menu.menuCode)), [existingCodes, menus]);
  const groupOptions = useMemo(() => flattenTree(tree).filter((item) => item.menuType === "Group"), [tree]);
  const sourceOptions = useMemo(() => flattenTree(sourceTree), [sourceTree]);

  const selectedItem = flattenTree(tree).find((item) => item.menuCode === selectedMenuCode) ?? null;
  const descendantCodes = new Set(selectedItem ? flattenTree([selectedItem]).map((item) => item.menuCode) : []);
  const parentOptions = groupOptions.filter((item) => !descendantCodes.has(item.menuCode));
  const editSiblings = (editParent === ROOT_VALUE ? tree : groupOptions.find((item) => item.menuCode === editParent)?.children ?? []).filter((item) => item.menuCode !== selectedMenuCode);
  const menuUsageHelp = <div className="space-y-1.5">
    <p className="font-medium">{text("arrangement.menuUsageGroup", "分组", "Group")}</p>
    <p>{text("arrangement.menuUsageGroupHelp", "侧栏中的容器，可以放置子菜单。", "A sidebar container that can hold child menu items.")}</p>
    <p className="font-medium">{text("arrangement.menuUsageInternal", "内部页面", "Internal page")}</p>
    <p>{text("arrangement.menuUsageInternalHelp", "打开 Tron 中配置的系统页面。", "Opens a configured page inside Tron.")}</p>
    <p className="font-medium">{text("arrangement.menuUsageExternal", "外部链接", "External link")}</p>
    <p>{text("arrangement.menuUsageExternalHelp", "打开菜单定义中配置的网址。", "Opens the URL configured in the menu definition.")}</p>
    <p className="font-medium">{text("arrangement.menuUsageWidget", "面板组件", "Panel widget")}</p>
    <p>{text("arrangement.menuUsageWidgetHelp", "打开聊天侧栏中的会话面板，例如项目或最近会话。", "Opens a chat sidebar panel, such as projects or recent sessions.")}</p>
  </div>;
  const parentValueHelp = <div className="space-y-1.5">
    <p>{text("arrangement.parentChoiceHelp", "每个选项决定菜单在侧栏中的层级：", "Each choice sets the item's level in the sidebar:")}</p>
    <ul className="list-disc space-y-1 pl-4">
      <li><span className="font-medium">{text("arrangement.topLevel", "顶层", "Top level")}</span> — {text("arrangement.parentRootHelp", "菜单直接显示在侧栏根部。", "Shows the item at the sidebar root.")}</li>
      {parentOptions.map((item) => <li key={item.menuCode}><span className="font-medium">{item.translations[locale]}</span> — {text("arrangement.parentGroupHelp", "菜单显示为该分组的子菜单。", "Places the item under this group.")}</li>)}
    </ul>
  </div>;
  const orderValueHelp = <div className="space-y-1.5">
    <p>{text("arrangement.orderChoiceHelp", "每个选项决定菜单在同级项目中的位置：", "Each choice sets this item's position among its siblings:")}</p>
    <ul className="list-disc space-y-1 pl-4">
      {editSiblings.map((item) => <li key={item.menuCode}><span className="font-medium">{text("arrangement.before", `在「${item.translations[locale]}」之前`, `Before ${item.translations[locale]}`)}</span> — {text("arrangement.orderBeforeHelp", `排在「${item.translations[locale]}」之前。`, `Places this item before “${item.translations[locale]}”.`)}</li>)}
      <li><span className="font-medium">{text("arrangement.last", "放在最后", "Last")}</span> — {text("arrangement.orderLastHelp", "排在当前层级所有菜单之后。", "Places this item after all other items at this level.")}</li>
    </ul>
  </div>;
  const expansionValueHelp = <div className="space-y-1.5">
    <p>{text("arrangement.expansionChoiceHelp", "选择分组或面板打开子内容的方式：", "Choose how a group or panel opens its child content:")}</p>
    <ul className="list-disc space-y-1 pl-4">
      <li><span className="font-medium">{text("arrangement.inline", "在侧栏内展开", "Expand in sidebar")}</span> — {text("arrangement.expansionInlineHelp", "子菜单显示在当前分组下方。", "Shows child items below the group in the sidebar.")}</li>
      <li><span className="font-medium">{text("arrangement.flyout", "向右弹出", "Open to the right")}</span> — {text("arrangement.expansionFlyoutHelp", "子菜单显示在侧栏旁的弹出面板中。", "Shows child items in a flyout beside the sidebar.")}</li>
    </ul>
  </div>;

  useEffect(() => {
    if (!selectedItem) return;
    setEditParent(selectedItem.parentMenuCode ?? ROOT_VALUE);
    setEditDisplay(selectedItem.childDisplay);
    const siblings = selectedItem.parentMenuCode
      ? flattenTree(tree).find((item) => item.menuCode === selectedItem.parentMenuCode)?.children ?? []
      : tree;
    setEditPosition(String(siblings.findIndex((item) => item.menuCode === selectedItem.menuCode)));
  }, [selectedItem, tree]);

  useEffect(() => {
    if (!selectedItem) setSelectedMenuCode(tree[0]?.menuCode ?? "");
  }, [selectedItem, tree]);

  async function saveItem(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedItem || saving) return;
    setSaving(true);
    try {
      setTree(await updateNavigationMenuItem(selectedTreeCode, selectedItem.menuCode, {
        parentMenuCode: editParent === ROOT_VALUE ? null : editParent,
        position: Number(editPosition),
        childDisplay: editDisplay,
      }));
      window.dispatchEvent(new Event("tron:navigation-updated"));
      toast.success(text("arrangement.saved", "菜单编排已保存", "Menu arrangement saved"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.moveItem", "菜单更新失败", "Menu could not be updated"));
    } finally {
      setSaving(false);
    }
  }

  function changeTree(treeCode: string) {
    setTree([]);
    setSelectedMenuCode("");
    setSelectedTreeCode(treeCode);
    navigate(`/admin/navigation/menu-tree?treeCode=${encodeURIComponent(treeCode)}`);
  }

  function openAddMenu() {
    setNewMenuCode("");
    setNewParentMenuCode(selectedItem?.menuType === "Group" ? selectedItem.menuCode : selectedItem?.parentMenuCode ?? ROOT_VALUE);
    setPosition(String(selectedItem?.menuType === "Group" ? selectedItem.children.length : tree.length));
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
      window.dispatchEvent(new Event("tron:navigation-updated"));
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
    if (!selectedTreeCode || saving) return;
    setSaving(true);
    try {
      const nextTree = await updateNavigationMenuItem(selectedTreeCode, item.menuCode, { parentMenuCode: parent, position: nextPosition, childDisplay: flattenTree(tree).find((node) => node.menuCode === item.menuCode)?.childDisplay ?? item.childDisplay });
      setTree(nextTree);
      window.dispatchEvent(new Event("tron:navigation-updated"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("errors.moveItem", "菜单移动失败", "Menu could not be moved"));
    } finally {
      setSaving(false);
    }
  }

  async function confirmRemoveMenu() {
    if (!selectedTreeCode || !itemToRemove) return;
    setSaving(true);
    try {
      const nextTree = await deleteNavigationMenuItem(selectedTreeCode, itemToRemove.menuCode);
      setTree(nextTree);
      window.dispatchEvent(new Event("tron:navigation-updated"));
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
      window.dispatchEvent(new Event("tron:navigation-updated"));
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
      window.dispatchEvent(new Event("tron:navigation-updated"));
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
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title={text("tree.title", "菜单编排", "Menu arrangement")} actions={<>
        <Button disabled={loading || saving} onClick={() => setRefreshKey((value) => value + 1)}>{text("actions.refresh", "刷新", "Refresh")}</Button>
        <Button disabled={!isApiMode || saving} onClick={() => setRestoreDefaultTreesOpen(true)}>{text("arrangement.restore", "补齐默认菜单", "Add missing defaults")}</Button>
        <Button disabled={!selectedTreeCode || saving || loading || menuTrees.length < 2} onClick={openCopyMenu}>{text("arrangement.copy", "从其他领域复制", "Copy from another domain")}</Button>
        <Button disabled={!selectedTreeCode || saving || loading} onClick={openAddMenu}>{text("arrangement.add", "添加", "Add")}</Button>
      </>} />
      <SurfaceCard className="min-h-0 flex-1 !p-0 overflow-hidden">
        <div className="grid h-full min-h-0 lg:grid-cols-[minmax(0,1fr)_20rem]">
          <section className="relative flex min-h-0 min-w-0 flex-col border-b border-[var(--color-outline-variant)] lg:border-b-0 lg:border-r" aria-label={text("arrangement.structure", "菜单结构", "Menu structure")}>
            <div className="flex min-h-14 shrink-0 items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-2">
              <div className="flex min-w-0 items-center gap-3">
                <h2 className="shrink-0 text-sm font-semibold">{text("arrangement.structure", "菜单结构", "Menu structure")}</h2>
                <SelectInput className="w-56" aria-label={text("arrangement.domain", "菜单领域", "Menu domain")} disabled={saving} value={selectedTreeCode} onValueChange={(value) => { if (value) changeTree(value); }}>{menuTrees.map((menuTree) => <SelectItem key={menuTree.treeCode} value={menuTree.treeCode}>{menuTree.name[locale]}</SelectItem>)}</SelectInput>
              </div>
              <span className="text-xs text-[var(--color-on-surface-variant)]">{text("arrangement.total", `${existingCodes.size} 个节点`, `${existingCodes.size} items`)}</span>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto p-3">
              {loading ? <p className="p-6 text-sm text-[var(--color-on-surface-variant)]">{text("loadingTree", "正在加载菜单树…", "Loading menu tree…")}</p> : tree.length ? <ul className="space-y-0.5">{tree.map((item, index) => <TreeNode key={item.menuCode} item={item} level={0} siblingIndex={index} siblingCount={tree.length} treeItems={tree} draggedCode={draggedCode} onDragChange={setDraggedCode} onMove={moveMenu} onSelect={setSelectedMenuCode} selectedCode={selectedMenuCode} busy={saving} locale={locale} text={text} />)}</ul> : <p className="p-6 text-sm text-[var(--color-on-surface-variant)]">{text("arrangement.empty", "加入菜单，开始编排这个领域。", "Add a menu to arrange this domain.")}</p>}
            </div>
            {draggedCode && !saving && (
              <div
                onDragOver={(event) => {
                  event.preventDefault();
                  event.dataTransfer.dropEffect = "move";
                  setRootDropActive(true);
                }}
                onDragLeave={(event) => {
                  if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setRootDropActive(false);
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  const source = flattenTree(tree).find((item) => item.menuCode === draggedCode);
                  setDraggedCode(null);
                  setRootDropActive(false);
                  if (!source) return;
                  const position = tree.filter((item) => item.menuCode !== source.menuCode).length;
                  if (source.parentMenuCode === null && tree[tree.length - 1]?.menuCode === source.menuCode) return;
                  void moveMenu(source, null, position);
                }}
                className={`absolute inset-x-3 bottom-3 z-30 flex min-h-14 items-center justify-center gap-2 rounded-lg border-2 border-dashed px-4 py-3 text-sm font-semibold shadow-lg transition-colors ${rootDropActive ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-on-primary)]" : "border-[var(--color-primary)] bg-[var(--color-elevated-surface)] text-[var(--color-primary)]"}`}
              >
                <FolderTree size={18} aria-hidden="true" />
                {locale === "zh-CN" ? "放在这里，移到顶层末尾" : "Drop here to move to the end of the top level"}
              </div>
            )}
          </section>
          <aside className="min-h-0 min-w-0 overflow-y-auto" aria-label={text("arrangement.settings", "节点设置", "Item settings")}>
            <div className="flex h-14 items-center gap-2 border-b border-[var(--color-outline-variant)] px-5 text-sm font-semibold"><Settings2 size={15} />{text("arrangement.settings", "节点设置", "Item settings")}</div>
            {selectedItem && !loading ? <form className="space-y-5 p-5" onSubmit={(event) => void saveItem(event)}>
              <div className="space-y-2">
                <div className="flex items-center gap-1"><h3 className="font-semibold">{selectedItem.translations[locale]}</h3><InfoTooltip ariaLabel={text("arrangement.menuUsageHelpAria", "查看菜单用途说明", "Show menu usage help")}>{menuUsageHelp}</InfoTooltip></div>
                <div className="flex gap-2"><Badge>{menuTypeLabel(selectedItem.menuType, text)}</Badge>{selectedItem.source === "BuiltIn" ? <Badge variant="info">{text("status.builtIn", "内置", "Built-in")}</Badge> : null}</div>
                <p className="break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{selectedItem.menuCode}</p>
                {selectedItem.description[locale] ? <p className="whitespace-pre-wrap break-words text-sm text-[var(--color-on-surface-variant)]">{selectedItem.description[locale]}</p> : null}
              </div>
              <Field label={text("fields.parent", "父级分组", "Parent group")} hint={parentValueHelp} hintAriaLabel={text("arrangement.parentHelpAria", "查看父级分组说明", "Show parent group help")}><SelectInput value={editParent} disabled={saving} onValueChange={(value) => { if (value) { setEditParent(value); setEditPosition("0"); } }}><SelectItem value={ROOT_VALUE}>{text("arrangement.topLevel", "顶层", "Top level")}</SelectItem>{parentOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
              <Field label={text("arrangement.order", "显示顺序", "Display order")} hint={orderValueHelp} hintAriaLabel={text("arrangement.orderHelpAria", "查看显示顺序说明", "Show display order help")}><SelectInput value={editPosition} disabled={saving} onValueChange={(value) => { if (value !== null) setEditPosition(value); }}>{editSiblings.map((item, index) => <SelectItem key={item.menuCode} value={String(index)}>{text("arrangement.before", `在「${item.translations[locale]}」之前`, `Before ${item.translations[locale]}`)}</SelectItem>)}<SelectItem value={String(editSiblings.length)}>{text("arrangement.last", "放在最后", "Last")}</SelectItem></SelectInput></Field>
              {(selectedItem.menuType === "Group" || selectedItem.menuType === "Widget") ? <Field label={text("arrangement.interaction", "展开方式", "Expansion")} hint={expansionValueHelp} hintAriaLabel={text("arrangement.expansionHelpAria", "查看展开方式说明", "Show expansion help")}><SelectInput value={editDisplay} disabled={saving} onValueChange={(value) => { if (value) setEditDisplay(value as "Inline" | "Flyout"); }}><SelectItem value="Inline">{text("arrangement.inline", "在侧栏内展开", "Expand in sidebar")}</SelectItem><SelectItem value="Flyout">{text("arrangement.flyout", "向右弹出", "Open to the right")}</SelectItem></SelectInput></Field> : <div className="space-y-1 text-sm"><p className="text-[var(--color-on-surface-variant)]">{text("columns.target", "目标", "Target")}</p><p className="break-all font-mono text-xs">{selectedItem.pageCode ?? selectedItem.url}</p></div>}
              <Button type="button" variant="secondary" className="w-full" onClick={() => navigate(`/admin/navigation/menus?menuCode=${encodeURIComponent(selectedItem.menuCode)}`)}>{text("arrangement.editContent", "编辑菜单内容", "Edit menu content")}</Button>
              <div className="flex items-center justify-between gap-2 border-t border-[var(--color-outline-variant)] pt-4">
                <Button type="button" variant="ghost" disabled={saving || selectedItem.children.length > 0} onClick={() => setItemToRemove(selectedItem)}><Trash2 size={14} />{text("actions.remove", "移除", "Remove")}</Button>
                <Button type="submit" disabled={saving} aria-busy={saving}>{text("actions.save", "保存", "Save")}</Button>
              </div>
            </form> : <p className="p-5 text-sm text-[var(--color-on-surface-variant)]">{text("arrangement.select", "选择一个菜单，调整内容、位置和交互。", "Select a menu to edit its content, position and interaction.")}</p>}
          </aside>
        </div>
      </SurfaceCard>

      <ModalDialog open={addMenuOpen} title={text("tree.addTitle", "加入菜单", "Add menu")} className="max-h-[88vh] max-w-xl overflow-y-auto">
          <form className="space-y-4" onSubmit={(event) => void addMenu(event)}>
            <Field label={text("fields.menu", "菜单", "Menu")} required><SelectInput value={newMenuCode} placeholder={text("tree.selectMenu", "选择菜单定义", "Select a menu definition")} onValueChange={(value) => { if (value) setNewMenuCode(value); }}>{availableMenus.length ? availableMenus.map((menu) => <SelectItem key={menu.menuCode} value={menu.menuCode}>{menu.translations[locale]} · {menu.menuCode}</SelectItem>) : <SelectItem disabled value="__NO_AVAILABLE_MENU__">{text("tree.noAvailableMenus", "暂无可加入的菜单", "No menus available")}</SelectItem>}</SelectInput></Field>
            <Field label={text("fields.parent", "父级分组", "Parent group")}><SelectInput value={newParentMenuCode} onValueChange={(value) => { if (value) setNewParentMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.root", "根节点", "Root")}</SelectItem>{groupOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.position", "位置", "Position")}><TextInput type="number" min="0" value={position} onChange={(event) => setPosition(event.target.value)} /></Field>
            <div className="flex justify-end gap-2"><Button type="button" variant="ghost" disabled={saving} onClick={() => setAddMenuOpen(false)}>{text("actions.cancel", "取消", "Cancel")}</Button><Button type="submit" disabled={!newMenuCode || saving} aria-busy={saving}>{text("actions.add", "加入", "Add")}</Button></div>
          </form>
      </ModalDialog>

      <ModalDialog open={copyMenuOpen} title={text("tree.copyTitle", "跨菜单树复制", "Copy across menu trees")} className="max-h-[88vh] max-w-xl overflow-y-auto">
          <form className="space-y-4" onSubmit={(event) => void copyMenuTree(event)}>
            <Field label={text("fields.sourceTree", "源菜单树", "Source menu tree")} required><SelectInput value={sourceTreeCode} onValueChange={(value) => { if (value) setSourceTreeCode(value); }}>{menuTrees.map((menuTree) => <SelectItem key={menuTree.treeCode} value={menuTree.treeCode}>{menuTree.name[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.sourceMenu", "源菜单", "Source menu")} hint={text("tree.copyHint", "目标菜单树存在同编码菜单时，整个复制操作会失败并回滚。", "If any menu code already exists in the target menu tree, the whole copy is rejected and rolled back.")}><SelectInput value={sourceMenuCode} onValueChange={(value) => { if (value) setSourceMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.wholeTree", "完整菜单树", "Entire menu tree")}</SelectItem>{sourceOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]} · {item.menuCode}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.targetParent", "目标父级分组", "Target parent group")}><SelectInput value={targetParentMenuCode} onValueChange={(value) => { if (value) setTargetParentMenuCode(value); }}><SelectItem value={ROOT_VALUE}>{text("tree.root", "根节点", "Root")}</SelectItem>{groupOptions.map((item) => <SelectItem key={item.menuCode} value={item.menuCode}>{item.translations[locale]}</SelectItem>)}</SelectInput></Field>
            <Field label={text("fields.position", "位置", "Position")}><TextInput type="number" min="0" value={copyPosition} onChange={(event) => setCopyPosition(event.target.value)} /></Field>
            <div className="flex justify-end gap-2"><Button type="button" variant="ghost" disabled={saving} onClick={() => setCopyMenuOpen(false)}>{text("actions.cancel", "取消", "Cancel")}</Button><Button type="submit" disabled={!sourceTreeCode || saving} aria-busy={saving}>{text("actions.copy", "复制", "Copy")}</Button></div>
          </form>
      </ModalDialog>
      <ConfirmDialog open={restoreDefaultTreesOpen} title={text("tree.restoreDefaultTrees", "生成所有默认菜单树", "Generate all default menu trees")} description={text("tree.restoreDefaultTreesDescription", "会补齐所有缺失的内置默认菜单节点；已有菜单及自定义编排不会被删除或覆盖。", "This adds every missing built-in default menu node. Existing menus and custom arrangements are not deleted or overwritten.")} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.generate", "生成", "Generate")} pending={saving} onCancel={() => setRestoreDefaultTreesOpen(false)} onConfirm={() => void confirmRestoreDefaultMenuTrees()} />
      <ConfirmDialog open={Boolean(itemToRemove)} title={text("tree.removeTitle", "从编排移除菜单", "Remove menu from arrangement")} description={text("tree.removeDescription", `确定移除 ${itemToRemove?.menuCode ?? ""}？这只会删除当前菜单树的编排关系，不会删除菜单定义。`, `Remove ${itemToRemove?.menuCode ?? ""}? This deletes only the current menu-tree arrangement; the menu definition remains.`)} cancelText={text("actions.cancel", "取消", "Cancel")} confirmText={text("actions.remove", "移除", "Remove")} pending={saving} onCancel={() => setItemToRemove(null)} onConfirm={() => void confirmRemoveMenu()} />
    </div>
  );
}
