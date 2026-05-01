import { useState, type ReactNode } from "react";
import {
  Bell,
  Bot,
  ChevronDown,
  CircleUserRound,
  Command,
  Database,
  FileClock,
  FolderGit2,
  Gauge,
  GitBranch,
  KeyRound,
  LayoutDashboard,
  LockKeyhole,
  MessageSquareText,
  Network,
  PanelLeftClose,
  PanelLeftOpen,
  PackageOpen,
  ScrollText,
  Search,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  UsersRound,
  Workflow,
  Wrench,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export interface TronNavItem {
  label: string;
  href: string;
  icon: LucideIcon;
}

export interface TronNavSection {
  label: string;
  items: TronNavItem[];
}

export const tronNavigation: TronNavSection[] = [
  {
    label: "Workspace",
    items: [
      { label: "Dashboard", href: "/dashboard", icon: LayoutDashboard },
      { label: "Apps", href: "/apps", icon: FolderGit2 },
      { label: "Tasks", href: "/tasks", icon: GitBranch },
      { label: "Automations", href: "/automations", icon: Workflow },
    ],
  },
  {
    label: "Agent Operations",
    items: [
      { label: "Chats", href: "/chats", icon: MessageSquareText },
      { label: "Prompts", href: "/prompts", icon: ScrollText },
      { label: "Models", href: "/models", icon: Sparkles },
      { label: "Skills", href: "/skills", icon: Wrench },
      { label: "Resources", href: "/resources", icon: PackageOpen },
    ],
  },
  {
    label: "Data",
    items: [
      { label: "Memory", href: "/memory", icon: Database },
      { label: "Graph", href: "/knowledge-graph", icon: Network },
      { label: "Variables", href: "/variables", icon: KeyRound },
      { label: "Observability", href: "/observability", icon: Gauge },
    ],
  },
  {
    label: "Administration",
    items: [
      { label: "Users", href: "/admin/users", icon: UsersRound },
      { label: "Roles", href: "/admin/roles", icon: LockKeyhole },
      { label: "Audit Logs", href: "/admin/audit-logs", icon: FileClock },
      { label: "Workspace Admin", href: "/admin/workspace", icon: Settings2 },
    ],
  },
];

interface TronShellProps {
  activeItem?: string;
  children: ReactNode;
  contentClassName?: string;
  rightPanel?: ReactNode;
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => item.label === activeItem));
}

export function TronShell({
  activeItem = "Dashboard",
  children,
  contentClassName,
  rightPanel,
  searchPlaceholder = "Search Tron...",
  topbarActions,
}: TronShellProps) {
  const [openSections, setOpenSections] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(tronNavigation.map((section) => [section.label, sectionHasActiveItem(section, activeItem) || section.label === "Workspace"])),
  );
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [utilityPanel, setUtilityPanel] = useState<string | null>(null);
  const [agentOpen, setAgentOpen] = useState(false);

  function toggleUtilityPanel(panel: string) {
    setUtilityPanel((current) => (current === panel ? null : panel));
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <header className="fixed inset-x-0 top-0 z-50 h-14 border-b border-[var(--color-outline-variant)] bg-[rgba(253,248,248,0.92)] px-4 backdrop-blur sm:px-6">
        <div className="flex h-full items-center justify-between gap-4">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <a className="flex shrink-0 items-center gap-2" href="/dashboard" aria-label="Tron dashboard">
              <Command size={19} aria-hidden="true" />
              <span className="text-lg font-semibold tracking-normal">Tron</span>
            </a>
            <Button
              className="hidden lg:inline-flex"
              size="icon"
              type="button"
              variant="ghost"
              aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
              aria-pressed={sidebarCollapsed}
              onClick={() => setSidebarCollapsed((current) => !current)}
            >
              {sidebarCollapsed ? <PanelLeftOpen size={17} aria-hidden="true" /> : <PanelLeftClose size={17} aria-hidden="true" />}
            </Button>
            <div className="flex h-9 min-w-[180px] max-w-md flex-1 items-center gap-2 rounded-full border border-[var(--color-outline-variant)] bg-white px-3 text-sm text-[var(--color-on-surface-variant)]">
              <Search size={16} aria-hidden="true" />
              <input className="min-w-0 flex-1 border-0 bg-transparent outline-none" placeholder={searchPlaceholder} type="search" />
            </div>
            <button
              className="hidden h-9 items-center gap-2 rounded-lg bg-[var(--color-surface-container)] px-3 text-sm font-medium text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)] md:flex"
              type="button"
              aria-expanded={utilityPanel === "workspace"}
              onClick={() => toggleUtilityPanel("workspace")}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              Enterprise Workspace
              <ChevronDown size={15} aria-hidden="true" />
            </button>
          </div>
          <div className="flex items-center gap-2">
            {topbarActions}
            <Button size="icon" variant="ghost" aria-label="Open filters" aria-pressed={utilityPanel === "filters"} onClick={() => toggleUtilityPanel("filters")}>
              <SlidersHorizontal size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="ghost" aria-label="View notifications" aria-pressed={utilityPanel === "notifications"} onClick={() => toggleUtilityPanel("notifications")}>
              <Bell size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="secondary" aria-label="Open user menu" aria-pressed={utilityPanel === "user"} onClick={() => toggleUtilityPanel("user")}>
              <CircleUserRound size={18} aria-hidden="true" />
            </Button>
          </div>
        </div>
        {utilityPanel ? (
          <div className="absolute right-4 top-[calc(100%+8px)] w-[min(320px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-white px-4 py-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)] sm:right-6">
            <p className="font-medium capitalize">{utilityPanel}</p>
            <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">Shared shell menu state is active. Page implementations can replace this with domain actions.</p>
          </div>
        ) : null}
      </header>

      <aside
        className={cn(
          "fixed bottom-0 left-0 top-14 z-40 hidden flex-col overflow-y-auto border-r border-[var(--color-outline-variant)] bg-white py-4 transition-[width] duration-200 lg:flex",
          sidebarCollapsed ? "w-[76px] px-2" : "w-[280px] px-3",
        )}
        data-collapsed={sidebarCollapsed}
      >
        <div className={cn("mb-6 flex items-center gap-3 px-2", sidebarCollapsed ? "justify-center" : "")}>
          <a className={cn("flex items-center gap-3 rounded-lg", sidebarCollapsed ? "justify-center" : "")} href="/dashboard" aria-label="Tron main console">
            <span className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-primary)] text-[var(--color-on-primary)]">
              <Command size={18} aria-hidden="true" />
            </span>
            <span className={cn(sidebarCollapsed ? "sr-only" : "")}>
              <span className="block text-sm font-semibold">Tron</span>
              <span className="block text-xs text-[var(--color-on-surface-variant)]">Main Console</span>
            </span>
          </a>
        </div>

        <nav className={cn("space-y-4", sidebarCollapsed ? "space-y-2" : "")} aria-label="Tron primary navigation">
          {tronNavigation.map((section) => {
            const open = openSections[section.label];
            return (
              <section key={section.label}>
                {sidebarCollapsed ? null : (
                  <button
                    className="flex h-8 w-full items-center justify-between rounded-lg px-2 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]"
                    type="button"
                    aria-expanded={open}
                    onClick={() => setOpenSections((current) => ({ ...current, [section.label]: !open }))}
                  >
                    {section.label}
                    <ChevronDown className={cn("transition-transform", open ? "rotate-180" : "")} size={14} aria-hidden="true" />
                  </button>
                )}
                {open || sidebarCollapsed ? (
                  <div className={cn("space-y-1", sidebarCollapsed ? "" : "mt-1")}>
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      const active = item.label === activeItem;
                      return (
                        <a
                          className={cn(
                            "flex h-9 items-center rounded-lg text-sm transition-colors",
                            sidebarCollapsed ? "justify-center px-0" : "gap-2 px-3",
                            active
                              ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                              : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                          )}
                          aria-current={active ? "page" : undefined}
                          data-active={active}
                          href={item.href}
                          key={item.label}
                          title={sidebarCollapsed ? item.label : undefined}
                        >
                          <Icon size={17} aria-hidden="true" />
                          <span className={cn(sidebarCollapsed ? "sr-only" : "")}>{item.label}</span>
                        </a>
                      );
                    })}
                  </div>
                ) : null}
              </section>
            );
          })}
        </nav>
      </aside>

      <section className={cn("min-w-0 pt-14 transition-[padding-left] duration-200", sidebarCollapsed ? "lg:pl-[76px]" : "lg:pl-[280px]")}>
        <div className={cn("grid min-h-[calc(100vh-3.5rem)] grid-cols-1", rightPanel ? "xl:grid-cols-[minmax(0,1fr)_360px]" : "")}>
          <main className={cn("min-w-0 px-6 py-6", contentClassName)}>{children}</main>
          {rightPanel ? <aside className="border-l border-[var(--color-outline-variant)] bg-white px-5 py-6">{rightPanel}</aside> : null}
        </div>
      </section>

      <button
        className="fixed bottom-6 right-6 z-40 flex h-12 items-center gap-2 rounded-full bg-[var(--color-primary)] px-4 text-sm font-medium text-[var(--color-on-primary)] shadow-[0_18px_45px_rgba(0,0,0,0.18)] transition-transform hover:scale-[1.02] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
        type="button"
        aria-expanded={agentOpen}
        onClick={() => setAgentOpen((current) => !current)}
      >
        <Bot size={18} aria-hidden="true" />
        Main Agent
      </button>
      {agentOpen ? (
        <section className="fixed bottom-24 right-6 z-40 w-[320px] rounded-2xl border border-[var(--color-card-border)] bg-white p-4 shadow-[0_18px_50px_rgba(0,0,0,0.14)]">
          <p className="text-sm font-semibold">Main Agent</p>
          <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">Contextual assistant entry is open. Step 3 pages can bind route-specific prompts here.</p>
          <button className="mt-3 h-8 rounded-lg bg-[var(--color-surface-container)] px-3 text-[13px] font-medium hover:bg-[var(--color-hover-surface)]" type="button" onClick={() => setAgentOpen(false)}>
            Close
          </button>
        </section>
      ) : null}
    </div>
  );
}
