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
  navigation: TronNavSection[];
  onNavigate: () => void;
  onToggleSection: (sectionLabel: string) => void;
  openNavigationItems: Record<string, boolean>;
  openSections: Record<string, boolean>;
  renderDomainSelector: (compact: boolean, mobile: boolean) => ReactNode;
  renderFooter: (compact: boolean, mobile: boolean) => ReactNode;
  renderChatPanel?: (compact: boolean) => ReactNode;
  setOpenNavigationItems: Dispatch<SetStateAction<Record<string, boolean>>>;
  translate: (key: string, options?: Record<string, string>) => string;
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
      <SidebarMenuItem className="space-y-1" key={itemKey}>
        {hasChildren && compact ? (
          <DropdownMenu>
            <DropdownMenuTrigger
              render={(
                <SidebarMenuButton
                  isActive={active}
                  tooltip={itemLabel}
                  className="justify-center px-0 mb-0"
                  aria-label={itemLabel}
                >
                  <Icon className="shrink-0" size={17} aria-hidden="true" />
                  <span className="sr-only">{itemLabel}</span>
                </SidebarMenuButton>
              )}
            />
            <DropdownMenuContent className="w-60 rounded-lg p-1.5" side="right" sideOffset={8}>
              <DropdownMenuGroup>
                <DropdownMenuLabel className="px-2 py-1.5 text-sm font-semibold text-[var(--color-on-surface)]">
                  {itemLabel}
                </DropdownMenuLabel>
                <DropdownMenuSeparator className="mx-0 my-1 bg-[var(--color-card-border)]" />
                {item.children?.map((child) => {
                  const ChildIcon = child.icon;
                  const childActive = child.label === activeItem || Boolean(child.href && activeHref && (child.href === activeHref || activeHref.startsWith(`${child.href}/`)));
                  const childLabel = child.translationKey ? translate(child.translationKey) : child.label;

                  return (
                    <DropdownMenuItem
                      className={cn(
                        "h-9 gap-[12px] rounded-md px-2 text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                        childActive && "bg-[var(--color-hover-surface)] font-medium text-[var(--color-on-surface)]",
                      )}
                      key={`${itemKey}/${child.label}`}
                      render={child.external
                        ? <a aria-current={childActive ? "page" : undefined} href={child.href} target="_blank" rel="noreferrer" />
                        : <Link aria-current={childActive ? "page" : undefined} to={child.href} />}
                    >
                      <ChildIcon size={16} aria-hidden="true" />
                      <span className="truncate">{childLabel}</span>
                    </DropdownMenuItem>
                  );
                })}
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
      aria-label={translate("shell:aria.primaryNavigation")}
      collapsible="icon"
      id="tron-sidebar"
      dir={undefined}
      className="pt-14"
    >
      <SidebarHeader className="gap-1.5 border-b border-[var(--color-card-border)] px-2 py-2.5">
        {renderDomainSelector(compact, isMobile)}
      </SidebarHeader>
      <SidebarContent className="p-2">
        {renderChatPanel ? renderChatPanel(compact) : null}
        <nav className={cn("flex flex-1 flex-col gap-[12px]", compact ? "gap-1" : "")} aria-label={translate("shell:aria.primaryNavigation")}>
          {navigation.map((section) => {
            const sectionKey = section.id ?? section.label;
            const open = openSections[sectionKey] ?? true;

            return (
              <SidebarGroup className="p-0" key={sectionKey}>
                {compact ? null : (
                  <SidebarGroupLabel
                    aria-expanded={open}
                    className="h-8 w-full justify-between px-2 text-[15px] font-semibold text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                    render={<button type="button" />}
                    onClick={() => onToggleSection(sectionKey)}
                  >
                    {section.translationKey ? translate(section.translationKey) : section.label}
                    <ChevronRight className={cn("transition-transform", open ? "rotate-90" : "")} size={14} aria-hidden="true" />
                  </SidebarGroupLabel>
                )}
                {open || compact ? (
                  <SidebarGroupContent>
                    <SidebarMenu className={cn(compact ? "" : "mt-1")}>
                      {section.items.map((item) => renderNavigationItem(item, `${section.label}/${item.label}`))}
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
