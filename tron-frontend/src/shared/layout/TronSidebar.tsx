import type {
  Dispatch,
  ReactNode,
  ReactElement,
  SetStateAction,
} from "react";
import {
  useEffect,
  useState,
} from "react";

import {
  ChevronDown,
  ChevronRight,
  Folder,
} from "lucide-react";
import { Link, useLocation } from "react-router";

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
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ChatNavigationWidget } from "./chat";
import { cn } from "@/shared/lib/utils";
import type {
  TronNavItem,
  TronNavSection,
} from "@/shared/types/ShellNavigation";

interface TronSidebarProps {
  activeItem: string;
  activeHref?: string;
  navigation: TronNavSection[];
  onNavigate: () => void;
  onToggleSection: (sectionLabel: string) => void;
  openNavigationItems: Record<string, boolean>;
  openSections: Record<string, boolean>;
  renderDomainSelector: (compact: boolean, mobile: boolean) => ReactNode;
  renderFooter: (compact: boolean, mobile: boolean) => ReactNode;
  setOpenNavigationItems: Dispatch<SetStateAction<Record<string, boolean>>>;
  translate: (key: string, options?: Record<string, string>) => string;
}

function SidebarFlyout({ label, trigger, children }: { label: string; trigger: ReactElement; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const location = useLocation();
  useEffect(() => setOpen(false), [location.pathname, location.search]);
  return <Popover open={open} onOpenChange={setOpen}>
    <PopoverTrigger render={trigger} />
    <PopoverContent side="right" align="start" sideOffset={8} aria-label={label} data-tron-sidebar-surface className="w-72" onClick={(event) => { if ((event.target as HTMLElement).closest("a")) setOpen(false); }}>
      {children}
    </PopoverContent>
  </Popover>;
}

export function TronSidebar({
  activeItem,
  activeHref,
  navigation,
  onNavigate,
  onToggleSection,
  openNavigationItems,
  openSections,
  renderDomainSelector,
  renderFooter,
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

  function renderNavigationItem(item: TronNavItem, itemKey: string, nestingLevel = 0, inFlyout = false): ReactNode {
    const itemCompact = compact && !inFlyout;
    if (item.widgetCode) {
      const widget = <ChatNavigationWidget menu={{ menuCode: item.menuCode ?? itemKey, pageCode: item.widgetCode, label: item.translationKey ? translate(item.translationKey) : item.label }} />;
      const WidgetIcon = item.icon;
      const widgetLabel = item.translationKey ? translate(item.translationKey) : item.label;
      return <SidebarMenuItem key={itemKey} className="list-none" data-menu-code={item.menuCode}>
        {itemCompact || item.childDisplay === "Flyout" ? <SidebarFlyout label={widgetLabel} trigger={<SidebarMenuButton aria-label={widgetLabel} tooltip={itemCompact ? widgetLabel : undefined} className={itemCompact ? "justify-center px-0" : "gap-3 px-3"}><WidgetIcon size={17} /><span className={cn("min-w-0 flex-1 truncate text-left", itemCompact && "sr-only")}>{widgetLabel}</span>{itemCompact ? null : <ChevronRight size={14} />}</SidebarMenuButton>}>{widget}</SidebarFlyout> : widget}
      </SidebarMenuItem>;
    }
    const Icon = item.icon;
    const active = item.label === activeItem || Boolean(item.href && activeHref && (item.href === activeHref || activeHref.startsWith(`${item.href}/`)));
    const itemLabel = item.translationKey ? translate(item.translationKey) : item.label;
    const hasChildren = item.isGroup || Boolean(item.children?.length);
    const openByDefault = !inFlyout;
    const open = hasChildren && (openNavigationItems[itemKey] ?? openByDefault);
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
        {hasChildren && (itemCompact || item.childDisplay === "Flyout") ? (
          <SidebarFlyout label={itemLabel} trigger={
            <SidebarMenuButton isActive={active} tooltip={itemCompact ? itemLabel : undefined} className={cn(itemCompact ? "justify-center px-0" : "gap-[12px] px-3", "group/nav-trigger mb-0")} aria-label={itemLabel}>
              <Icon className="shrink-0" size={17} aria-hidden="true" />
              <span className={cn("min-w-0 flex-1 truncate text-left", itemCompact ? "sr-only" : "")}>{itemLabel}</span>
              {itemCompact ? null : <ChevronRight className="shrink-0 transition-opacity [@media(hover:hover)]:opacity-0 group-hover/nav-trigger:opacity-60 group-focus-visible/nav-trigger:opacity-60 group-aria-expanded/nav-trigger:opacity-60" size={14} aria-hidden="true" />}
            </SidebarMenuButton>
          }>
            {item.children?.length ? <SidebarMenu>{item.children.map((child) => renderNavigationItem(child, `${itemKey}/${child.menuCode ?? child.label}`, 0, true))}</SidebarMenu> : <p className="px-2 py-3 text-sm text-muted-foreground">{translate("shell:navigation.emptyGroup")}</p>}
          </SidebarFlyout>
        ) : hasChildren ? (
          <SidebarMenuButton
            aria-expanded={open}
            aria-label={translate(open ? "shell:aria.collapseNavigationItem" : "shell:aria.expandNavigationItem", { item: itemLabel })}
            className="gap-[12px] px-3 text-sidebar-foreground"
            type="button"
            onClick={() => {
              setOpenNavigationItems((current) => ({ ...current, [itemKey]: !(current[itemKey] ?? openByDefault) }));
            }}
          >
            <Icon className="shrink-0" size={17} aria-hidden="true" />
            <span className="min-w-0 flex-1 truncate text-left">{itemLabel}</span>
            <ChevronDown className={cn("shrink-0 transition-transform", open ? "" : "-rotate-90")} size={14} aria-hidden="true" />
          </SidebarMenuButton>
        ) : (
          <SidebarMenuButton
            isActive={active}
            tooltip={itemCompact ? itemLabel : undefined}
            className={cn(itemCompact ? "justify-center px-0" : "gap-[12px] px-3")}
            render={item.external
              ? <a aria-current={active ? "page" : undefined} aria-label={itemCompact ? itemLabel : undefined} href={item.href} target="_blank" rel="noreferrer" />
              : <Link aria-current={active ? "page" : undefined} aria-label={itemCompact ? itemLabel : undefined} to={item.href} />}
            onClick={() => {
              if (isMobile) {
                setOpenMobile(false);
                onNavigate();
              }
            }}
          >
            <Icon className="shrink-0" size={17} aria-hidden="true" />
            <span className={cn("truncate", itemCompact ? "sr-only" : "")}>{itemLabel}</span>
          </SidebarMenuButton>
        )}
        {open && !itemCompact && item.childDisplay !== "Flyout" ? (
          <SidebarMenuSub className="m-0 border-none">
            {item.children?.map((child) => renderNavigationItem(child, `${itemKey}/${child.label}`, nestingLevel + 1, inFlyout))}
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
        <nav className={cn("flex flex-1 flex-col gap-[12px] p-2", compact ? "gap-1" : "")} aria-label={translate("shell:aria.primaryNavigation")} data-theme-part="app-sidebar-menu">
          {navigation.map((section) => {
            const sectionKey = section.id ?? section.label;
            const open = openSections[sectionKey] ?? true;
            const sectionItems: TronNavItem[] = section.flyout ? [{
              childDisplay: "Flyout" as const,
              isGroup: true,
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
