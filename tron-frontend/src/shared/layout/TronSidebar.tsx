import type {
  Dispatch,
  ReactNode,
  SetStateAction,
} from "react";
import {
  useEffect,
} from "react";

import {
  ChevronDown,
  ChevronRight,
  Folder,
} from "lucide-react";
import { Link } from "react-router";

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/shared/lib/utils";
import type {
  TronNavItem,
  TronNavSection,
} from "@/shared/types/ShellNavigation";

interface TronSidebarProps {
  activeItem: string;
  activeHref?: string;
  activeSessionCode: string;
  navigateToPath: (path: string) => void;
  navigation: TronNavSection[];
  onNavigate: () => void;
  onToggleSection: (sectionLabel: string) => void;
  openNavigationItems: Record<string, boolean>;
  openSections: Record<string, boolean>;
  renderDomainSelector: (compact: boolean, mobile: boolean) => ReactNode;
  renderFooter: (compact: boolean, mobile: boolean) => ReactNode;
  renderChatPanel?: (
    compact: boolean,
    activeSessionCode: string,
    navigateToPath: (path: string) => void,
  ) => ReactNode;
  setOpenNavigationItems: Dispatch<SetStateAction<Record<string, boolean>>>;
  translate: (key: string, options?: Record<string, string>) => string;
}

export function TronSidebar({
  activeItem,
  activeHref,
  activeSessionCode,
  navigateToPath,
  navigation,
  onNavigate,
  onToggleSection,
  openNavigationItems,
  openSections,
  renderDomainSelector,
  renderFooter,
  renderChatPanel,
  setOpenNavigationItems,
  translate,
}: TronSidebarProps) {
  const {
    isMobile,
    setOpenMobile,
    state,
    toggleSidebar,
  } = useSidebar();
  const compact = !isMobile && state === "collapsed";

  useEffect(() => {
    const toggle = () => toggleSidebar();
    const close = () => setOpenMobile(false);

    window.addEventListener("tron:toggle-sidebar", toggle);
    window.addEventListener("tron:close-sidebar", close);
    return () => {
      window.removeEventListener("tron:toggle-sidebar", toggle);
      window.removeEventListener("tron:close-sidebar", close);
    };
  }, [setOpenMobile, toggleSidebar]);

  function renderDropdownNavigationItem(item: TronNavItem, itemKey: string): ReactNode {
    const Icon = item.icon;
    const itemLabel = item.translationKey ? translate(item.translationKey) : item.label;
    const active = item.label === activeItem
      || Boolean(item.href && activeHref && (item.href === activeHref || activeHref.startsWith(`${item.href}/`)));
    if (item.children?.length) {
      return (
        <DropdownMenuSub key={itemKey}>
          <DropdownMenuSubTrigger className="h-9 gap-[12px] rounded-md px-2 text-[var(--color-on-surface-variant)]">
            <Icon size={16} aria-hidden="true" />
            <span className="truncate">{itemLabel}</span>
          </DropdownMenuSubTrigger>
          <DropdownMenuSubContent className="w-60 rounded-lg p-1.5">
            {item.children.map((child) => renderDropdownNavigationItem(child, `${itemKey}/${child.label}`))}
          </DropdownMenuSubContent>
        </DropdownMenuSub>
      );
    }

    return (
      <DropdownMenuItem
        className={cn(
          "h-9 gap-[12px] rounded-md px-2 text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
          active && "bg-[var(--color-hover-surface)] font-medium text-[var(--color-on-surface)]",
        )}
        key={itemKey}
        render={item.external
          ? <a aria-current={active ? "page" : undefined} href={item.href} target="_blank" rel="noreferrer" />
          : <Link aria-current={active ? "page" : undefined} to={item.href} />}
      >
        <Icon size={16} aria-hidden="true" />
        <span className="truncate">{itemLabel}</span>
      </DropdownMenuItem>
    );
  }

  function renderNavigationItem(item: TronNavItem, itemKey: string, nestingLevel = 0): ReactNode {
    const Icon = item.icon;
    const active = item.label === activeItem || Boolean(item.href && activeHref && (item.href === activeHref || activeHref.startsWith(`${item.href}/`)));
    const itemLabel = item.translationKey ? translate(item.translationKey) : item.label;
    const hasChildren = Boolean(item.children?.length);
    const open = hasChildren && openNavigationItems[itemKey];
    if (nestingLevel > 0 && !hasChildren) {
      return (
        <SidebarMenuSubItem key={itemKey}>
          <SidebarMenuSubButton
            isActive={active}
            render={item.external ? <a href={item.href} target="_blank" rel="noreferrer" /> : <Link to={item.href} />}
            onClick={() => {
              if (isMobile) {
                setOpenMobile(false);
                onNavigate();
              }
            }}
          >
            <Icon aria-hidden="true" />
            <span>{itemLabel}</span>
          </SidebarMenuSubButton>
        </SidebarMenuSubItem>
      );
    }

    return (
      <SidebarMenuItem className="space-y-1" data-theme-part="app-sidebar-menu-item" key={itemKey}>
        {hasChildren && (compact || item.childDisplay === "Flyout") ? (
          <DropdownMenu>
            <DropdownMenuTrigger
              render={(
                <SidebarMenuButton
                  isActive={active}
                  tooltip={itemLabel}
                  className={cn(compact ? "justify-center px-0" : "gap-[12px] px-3", "mb-0")}
                  aria-label={itemLabel}
                >
                  <Icon className="shrink-0" size={17} aria-hidden="true" />
                  <span className={cn("min-w-0 flex-1 truncate text-left", compact ? "sr-only" : "")}>{itemLabel}</span>
                  {compact ? null : <ChevronRight className="shrink-0" size={14} aria-hidden="true" />}
                </SidebarMenuButton>
              )}
            />
            <DropdownMenuContent className="w-60 rounded-lg p-1.5" side="right" sideOffset={8}>
              <DropdownMenuGroup>
                <DropdownMenuLabel className="px-2 py-1.5 text-sm font-semibold text-[var(--color-on-surface)]">
                  {itemLabel}
                </DropdownMenuLabel>
                <DropdownMenuSeparator className="mx-0 my-1 bg-[var(--color-card-border)]" />
                {item.children?.map((child) => renderDropdownNavigationItem(child, `${itemKey}/${child.label}`))}
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        ) : hasChildren ? (
          <SidebarMenuButton
            aria-expanded={open}
            aria-label={translate(open ? "shell:aria.collapseNavigationItem" : "shell:aria.expandNavigationItem", { item: itemLabel })}
            className="gap-[12px] px-3 text-sidebar-foreground"
            type="button"
            onClick={() => {
              setOpenNavigationItems((current) => ({ ...current, [itemKey]: !current[itemKey] }));
            }}
          >
            <Icon className="shrink-0" size={17} aria-hidden="true" />
            <span className="min-w-0 flex-1 truncate text-left">{itemLabel}</span>
            <ChevronDown className={cn("shrink-0 transition-transform", open ? "" : "-rotate-90")} size={14} aria-hidden="true" />
          </SidebarMenuButton>
        ) : (
          <SidebarMenuButton
            isActive={active}
            tooltip={compact ? itemLabel : undefined}
            className={cn(compact ? "justify-center px-0" : "gap-[12px] px-3")}
            render={item.external
              ? <a aria-current={active ? "page" : undefined} aria-label={compact ? itemLabel : undefined} href={item.href} target="_blank" rel="noreferrer" />
              : <Link aria-current={active ? "page" : undefined} aria-label={compact ? itemLabel : undefined} to={item.href} />}
            onClick={() => {
              if (isMobile) {
                setOpenMobile(false);
                onNavigate();
              }
            }}
          >
            <Icon className="shrink-0" size={17} aria-hidden="true" />
            <span className={cn("truncate", compact ? "sr-only" : "")}>{itemLabel}</span>
          </SidebarMenuButton>
        )}
        {open && !compact ? (
          <SidebarMenuSub className="m-0 border-none">
            {item.children?.map((child) => renderNavigationItem(child, `${itemKey}/${child.label}`, nestingLevel + 1))}
          </SidebarMenuSub>
        ) : null}
      </SidebarMenuItem>
    );
  }

  return (
    <Sidebar
      data-theme-part="app-sidebar"
      aria-label={translate("shell:aria.primaryNavigation")}
      collapsible="icon"
      id="tron-sidebar"
      dir={undefined}
      className="pt-14"
    >
      <SidebarHeader className="gap-1.5 border-b border-[var(--color-card-border)] px-2 py-2.5">
        {renderDomainSelector(compact, isMobile)}
      </SidebarHeader>
      <SidebarContent data-theme-part="app-sidebar-content">
        {renderChatPanel
          ? renderChatPanel(compact, activeSessionCode, navigateToPath)
          : null}
        <nav className={cn("flex flex-1 flex-col gap-[12px] p-2", compact ? "gap-1" : "")} aria-label={translate("shell:aria.primaryNavigation")} data-theme-part="app-sidebar-menu">
          {navigation.map((section) => {
            const sectionKey = section.id ?? section.label;
            const open = openSections[sectionKey] ?? true;
            const sectionItems: TronNavItem[] = section.flyout ? [{
              childDisplay: "Flyout" as const,
              children: section.items,
              href: "",
              icon: section.icon ?? section.items[0]?.icon ?? Folder,
              label: section.label,
              translationKey: section.translationKey,
            }] : section.items;

            return (
              <SidebarGroup className="p-0" key={sectionKey}>
                {compact || section.flyout || section.standalone ? null : (
                  <SidebarGroupLabel
                    aria-expanded={open}
                    className="h-8 w-full justify-between px-2 text-sm font-semibold text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                    render={<button type="button" />}
                    onClick={() => onToggleSection(sectionKey)}
                  >
                    {section.translationKey ? translate(section.translationKey) : section.label}
                    <ChevronRight className={cn("transition-transform", open ? "rotate-90" : "")} size={14} aria-hidden="true" />
                  </SidebarGroupLabel>
                )}
                {section.flyout || section.standalone || open || compact ? (
                  <SidebarGroupContent>
                    <SidebarMenu className={cn(compact ? "" : "mt-1")}>
                      {sectionItems.map((item) => renderNavigationItem(item, `${section.label}/${item.label}`))}
                    </SidebarMenu>
                  </SidebarGroupContent>
                ) : null}
              </SidebarGroup>
            );
          })}
        </nav>
      </SidebarContent>
      <SidebarFooter>{renderFooter(compact, isMobile)}</SidebarFooter>
    </Sidebar>
  );
}
