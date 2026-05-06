import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
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
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  UsersRound,
  Workflow,
  Wrench,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useLocale } from "@/i18n";
import { ClientLink } from "@/lib/client-router";
import { cn } from "@/lib/utils";

export interface TronNavItem {
  label: string;
  href: string;
  icon: LucideIcon;
  translationKey: string;
}

export interface TronNavSection {
  label: string;
  items: TronNavItem[];
  translationKey: string;
}

export const tronNavigation: TronNavSection[] = [
  {
    label: "Core",
    translationKey: "shell:nav.sections.core",
    items: [
      { label: "Dashboard", href: "/dashboard", icon: LayoutDashboard, translationKey: "shell:nav.items.dashboard" },
      { label: "Apps", href: "/apps", icon: FolderGit2, translationKey: "shell:nav.items.apps" },
      { label: "Tasks", href: "/tasks", icon: GitBranch, translationKey: "shell:nav.items.tasks" },
      { label: "Automations", href: "/automations", icon: Workflow, translationKey: "shell:nav.items.automations" },
    ],
  },
  {
    label: "Agent Operations",
    translationKey: "shell:nav.sections.agentOperations",
    items: [
      { label: "Chats", href: "/chats", icon: MessageSquareText, translationKey: "shell:nav.items.chats" },
      { label: "Prompts", href: "/prompts", icon: ScrollText, translationKey: "shell:nav.items.prompts" },
      { label: "Models", href: "/models", icon: Sparkles, translationKey: "shell:nav.items.models" },
      { label: "Skills", href: "/skills", icon: Wrench, translationKey: "shell:nav.items.skills" },
      { label: "Resources", href: "/resources", icon: PackageOpen, translationKey: "shell:nav.items.resources" },
    ],
  },
  {
    label: "Data",
    translationKey: "shell:nav.sections.data",
    items: [
      { label: "Memory", href: "/memory", icon: Database, translationKey: "shell:nav.items.memory" },
      { label: "Graph", href: "/knowledge-graph", icon: Network, translationKey: "shell:nav.items.graph" },
      { label: "Variables", href: "/variables", icon: KeyRound, translationKey: "shell:nav.items.variables" },
      { label: "Observability", href: "/observability", icon: Gauge, translationKey: "shell:nav.items.observability" },
    ],
  },
  {
    label: "Administration",
    translationKey: "shell:nav.sections.administration",
    items: [
      { label: "Users", href: "/admin/users", icon: UsersRound, translationKey: "shell:nav.items.users" },
      { label: "Roles", href: "/admin/roles", icon: LockKeyhole, translationKey: "shell:nav.items.roles" },
      { label: "Audit Logs", href: "/admin/audit-logs", icon: FileClock, translationKey: "shell:nav.items.auditLogs" },
      { label: "Workspace Admin", href: "/admin/workspace", icon: Settings2, translationKey: "shell:nav.items.workspaceAdmin" },
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

const TronShellFrameContext = createContext(false);
const defaultPrimarySpaceId = "ps-enterprise";
const defaultSecondarySpaceId = "ss-tron-main";

export interface PrimarySpace {
  id: string;
  name: string;
  description: string;
  region: string;
  status: "Active" | "Review" | "Archived";
}

export interface SecondarySpace {
  id: string;
  primarySpaceId: string;
  name: string;
  description: string;
  inheritedResources: number;
  localOverrides: number;
  status: "Active" | "Review" | "Disabled";
}

export const workspaceSpaceApiContracts = {
  listPrimarySpaces: "GET /api/v1/primary-spaces",
  listSecondarySpaces: "GET /api/v1/primary-spaces/{primarySpaceId}/secondary-spaces",
} as const;

const primarySpaceMocks: PrimarySpace[] = [
  {
    id: "ps-enterprise",
    name: "Enterprise Workspace",
    description: "Global governance, security, and shared AI resource policy.",
    region: "Global",
    status: "Active",
  },
  {
    id: "ps-research",
    name: "Research Workspace",
    description: "R&D model evaluation, prompt experiments, and internal prototypes.",
    region: "APAC",
    status: "Active",
  },
  {
    id: "ps-customer-ops",
    name: "Customer Ops Workspace",
    description: "Customer-facing agent operations and support workflows.",
    region: "US",
    status: "Active",
  },
  {
    id: "ps-compliance",
    name: "Compliance Workspace",
    description: "Audit, retention, permission, and regulated resource controls.",
    region: "EU",
    status: "Review",
  },
];

const secondarySpaceMocks: SecondarySpace[] = [
  { id: "ss-tron-main", primarySpaceId: "ps-enterprise", name: "Tron", description: "Main Console", inheritedResources: 24, localOverrides: 5, status: "Active" },
  { id: "ss-agent-platform", primarySpaceId: "ps-enterprise", name: "Agent Platform", description: "Runtime Governance", inheritedResources: 18, localOverrides: 3, status: "Active" },
  { id: "ss-security-lab", primarySpaceId: "ps-enterprise", name: "Security Lab", description: "Policy Sandbox", inheritedResources: 16, localOverrides: 7, status: "Review" },
  { id: "ss-model-research", primarySpaceId: "ps-research", name: "Model Research", description: "Evaluation Console", inheritedResources: 12, localOverrides: 8, status: "Active" },
  { id: "ss-prompt-lab", primarySpaceId: "ps-research", name: "Prompt Lab", description: "Experiment Studio", inheritedResources: 10, localOverrides: 11, status: "Active" },
  { id: "ss-support-ai", primarySpaceId: "ps-customer-ops", name: "Support AI", description: "Customer Agents", inheritedResources: 21, localOverrides: 4, status: "Active" },
  { id: "ss-success-ops", primarySpaceId: "ps-customer-ops", name: "Success Ops", description: "Renewal Workflows", inheritedResources: 15, localOverrides: 6, status: "Active" },
  { id: "ss-audit-center", primarySpaceId: "ps-compliance", name: "Audit Center", description: "Evidence Review", inheritedResources: 19, localOverrides: 2, status: "Review" },
];

function getSecondarySpaceMocks(primarySpaceId: PrimarySpace["id"]) {
  return secondarySpaceMocks.filter((space) => space.primarySpaceId === primarySpaceId);
}

interface WorkspaceSpacesContextValue {
  primarySpaces: PrimarySpace[];
  secondarySpaces: SecondarySpace[];
  selectedPrimarySpace?: PrimarySpace;
  selectedPrimarySpaceId: string;
  selectedSecondarySpace?: SecondarySpace;
  selectedSecondarySpaceId: string;
}

const defaultSecondarySpaces = getSecondarySpaceMocks(defaultPrimarySpaceId);

const WorkspaceSpacesContext = createContext<WorkspaceSpacesContextValue>({
  primarySpaces: primarySpaceMocks,
  secondarySpaces: defaultSecondarySpaces,
  selectedPrimarySpace: primarySpaceMocks.find((space) => space.id === defaultPrimarySpaceId),
  selectedPrimarySpaceId: defaultPrimarySpaceId,
  selectedSecondarySpace: defaultSecondarySpaces.find((space) => space.id === defaultSecondarySpaceId) ?? defaultSecondarySpaces[0],
  selectedSecondarySpaceId: defaultSecondarySpaceId,
});

export function useWorkspaceSpaces() {
  return useContext(WorkspaceSpacesContext);
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => item.label === activeItem));
}

function InlineTronShellContent({ children, contentClassName, rightPanel }: TronShellProps) {
  return (
    <div className={cn("-mx-6 -my-6 grid min-h-[calc(100vh-3.5rem)] grid-cols-1", rightPanel ? "xl:grid-cols-[minmax(0,1fr)_360px]" : "")}>
      <main className={cn("min-w-0 px-6 py-6", contentClassName)}>{children}</main>
      {rightPanel ? <aside className="border-l border-[var(--color-outline-variant)] bg-white px-5 py-6">{rightPanel}</aside> : null}
    </div>
  );
}

export function TronShell({
  activeItem = "Dashboard",
  children,
  contentClassName,
  rightPanel,
  searchPlaceholder,
  topbarActions,
}: TronShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const { changeLocale, currentLocale, supportedLocales: locales } = useLocale();
  const shellFrameMounted = useContext(TronShellFrameContext);
  const [openSections, setOpenSections] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(tronNavigation.map((section) => [section.label, sectionHasActiveItem(section, activeItem) || section.label === "Core"])),
  );
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [utilityPanel, setUtilityPanel] = useState<string | null>(null);
  const [agentOpen, setAgentOpen] = useState(false);
  const [selectedPrimarySpaceId, setSelectedPrimarySpaceId] = useState(defaultPrimarySpaceId);
  const [selectedSecondarySpaceId, setSelectedSecondarySpaceId] = useState(defaultSecondarySpaceId);
  const [secondarySpaceMenuOpen, setSecondarySpaceMenuOpen] = useState(false);

  const primarySpaces = primarySpaceMocks;
  const secondarySpaces = getSecondarySpaceMocks(selectedPrimarySpaceId);
  const selectedPrimarySpace = primarySpaces.find((space) => space.id === selectedPrimarySpaceId);
  const selectedSecondarySpace = secondarySpaces.find((space) => space.id === selectedSecondarySpaceId) ?? secondarySpaces[0];
  const workspaceSpacesValue = useMemo<WorkspaceSpacesContextValue>(
    () => ({
      primarySpaces,
      secondarySpaces,
      selectedPrimarySpace,
      selectedPrimarySpaceId,
      selectedSecondarySpace,
      selectedSecondarySpaceId: selectedSecondarySpace?.id ?? selectedSecondarySpaceId,
    }),
    [primarySpaces, secondarySpaces, selectedPrimarySpace, selectedPrimarySpaceId, selectedSecondarySpace, selectedSecondarySpaceId],
  );

  function toggleUtilityPanel(panel: string) {
    setSecondarySpaceMenuOpen(false);
    setUtilityPanel((current) => (current === panel ? null : panel));
  }

  function toggleSecondarySpaceMenu() {
    setUtilityPanel(null);
    setSecondarySpaceMenuOpen((current) => !current);
  }

  function selectPrimarySpace(spaceId: string) {
    setSelectedPrimarySpaceId(spaceId);
    setSelectedSecondarySpaceId(getSecondarySpaceMocks(spaceId)[0]?.id ?? "");
    setSecondarySpaceMenuOpen(false);
    setUtilityPanel(null);
  }

  function selectSecondarySpace(spaceId: string) {
    setSelectedSecondarySpaceId(spaceId);
    setSecondarySpaceMenuOpen(false);
  }

  useEffect(() => {
    const activeSection = tronNavigation.find((section) => sectionHasActiveItem(section, activeItem));
    if (!activeSection) return;

    setOpenSections((current) => (current[activeSection.label] ? current : { ...current, [activeSection.label]: true }));
  }, [activeItem]);

  if (shellFrameMounted) {
    return <InlineTronShellContent activeItem={activeItem} contentClassName={contentClassName} rightPanel={rightPanel}>{children}</InlineTronShellContent>;
  }

  return (
    <TronShellFrameContext.Provider value={true}>
    <WorkspaceSpacesContext.Provider value={workspaceSpacesValue}>
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <header className="fixed inset-x-0 top-0 z-50 h-14 border-b border-[var(--color-outline-variant)] bg-[rgba(253,248,248,0.92)] px-4 backdrop-blur sm:px-6">
        <div className="flex h-full items-center justify-between gap-4">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <button
              className="inline-flex h-9 shrink-0 items-center gap-2 rounded-lg bg-[var(--color-surface-container)] px-3 text-sm font-medium text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              aria-expanded={utilityPanel === "workspace"}
              onClick={() => toggleUtilityPanel("workspace")}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {selectedPrimarySpace?.name ?? t("shell:workspace.current")}
              <ChevronDown size={15} aria-hidden="true" />
            </button>
            <Button
              className="hidden lg:inline-flex"
              size="icon"
              type="button"
              variant="ghost"
              aria-label={sidebarCollapsed ? t("shell:aria.expandSidebar") : t("shell:aria.collapseSidebar")}
              aria-pressed={sidebarCollapsed}
              onClick={() => setSidebarCollapsed((current) => !current)}
            >
              {sidebarCollapsed ? <PanelLeftOpen size={17} aria-hidden="true" /> : <PanelLeftClose size={17} aria-hidden="true" />}
            </Button>
          </div>
          <div className="flex items-center gap-2">
            {topbarActions}
            <Button size="icon" variant="ghost" aria-label={t("shell:aria.openFilters")} aria-pressed={utilityPanel === "filters"} onClick={() => toggleUtilityPanel("filters")}>
              <SlidersHorizontal size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="ghost" aria-label={t("shell:aria.viewNotifications")} aria-pressed={utilityPanel === "notifications"} onClick={() => toggleUtilityPanel("notifications")}>
              <Bell size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="secondary" aria-label={t("shell:aria.openUserMenu")} aria-pressed={utilityPanel === "user"} onClick={() => toggleUtilityPanel("user")}>
              <CircleUserRound size={18} aria-hidden="true" />
            </Button>
          </div>
        </div>
        {utilityPanel ? (
          <div
            className={cn(
              "absolute top-[calc(100%+8px)] w-[min(360px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-white px-4 py-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              utilityPanel === "workspace" ? "left-4 sm:left-6" : "right-4 sm:right-6",
            )}
          >
            <p className="font-medium">{t(`shell:utilityPanel.${utilityPanel}`)}</p>
            {utilityPanel === "workspace" ? (
              <div className="mt-3 space-y-2">
                <p className="text-[12px] font-medium text-[var(--color-on-surface-variant)]">{t("shell:workspace.primaryLabel")}</p>
                {primarySpaces.length ? (
                  primarySpaces.map((space) => {
                    const selected = space.id === selectedPrimarySpaceId;
                    return (
                      <button
                        className={cn(
                          "block w-full rounded-lg border px-3 py-2 text-left transition-colors",
                          selected ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]" : "border-transparent hover:bg-[var(--color-hover-surface)]",
                        )}
                        type="button"
                        aria-pressed={selected}
                        key={space.id}
                        onClick={() => selectPrimarySpace(space.id)}
                      >
                        <span className="block text-sm font-medium">{space.name}</span>
                        <span className="mt-0.5 block text-[12px] text-[var(--color-on-surface-variant)]">{space.description}</span>
                        <span className="mt-1 block text-[11px] text-[var(--color-on-surface-variant)]">{space.region} · {t("shell:workspace.status", { status: space.status })}</span>
                      </button>
                    );
                  })
                ) : (
                  <p className="text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.noPrimarySpaces")}</p>
                )}
              </div>
            ) : utilityPanel === "user" ? (
              <div className="mt-3">
                <label className="mb-1 block text-[12px] font-medium text-[var(--color-on-surface-variant)]" htmlFor="tron-locale-select">
                  {t("common:locale.label")}
                </label>
                <select
                  className="h-9 w-full rounded-lg border border-[var(--color-outline-variant)] bg-white px-3 text-sm outline-none"
                  id="tron-locale-select"
                  value={currentLocale}
                  onChange={(event) => void changeLocale(event.target.value as typeof currentLocale)}
                >
                  {locales.map((locale) => (
                    <option key={locale.code} value={locale.code}>
                      {t(locale.labelKey)}
                    </option>
                  ))}
                </select>
                <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">{t("common:locale.unavailable")}</p>
              </div>
            ) : (
              <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:utilityPanel.sharedState")}</p>
            )}
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
          <button
            className={cn("flex w-full items-center gap-3 rounded-lg text-left", sidebarCollapsed ? "justify-center" : "")}
            type="button"
            aria-expanded={secondarySpaceMenuOpen}
            aria-label={selectedSecondarySpace ? `${selectedSecondarySpace.name} ${selectedSecondarySpace.description}` : t("shell:aria.tronMainConsole")}
            onClick={toggleSecondarySpaceMenu}
          >
            <span className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-primary)] text-[var(--color-on-primary)]">
              <Command size={18} aria-hidden="true" />
            </span>
            <span className={cn(sidebarCollapsed ? "sr-only" : "")}>
              <span className="block text-sm font-semibold">{selectedSecondarySpace?.name ?? t("common:brand.name")}</span>
              <span className="block text-xs text-[var(--color-on-surface-variant)]">{selectedSecondarySpace?.description ?? t("common:brand.console")}</span>
            </span>
            {sidebarCollapsed ? null : <ChevronDown className={cn("ml-auto transition-transform", secondarySpaceMenuOpen ? "rotate-180" : "")} size={14} aria-hidden="true" />}
          </button>
        </div>

        <nav className={cn("space-y-4", sidebarCollapsed ? "space-y-2" : "")} aria-label={t("shell:aria.primaryNavigation")}>
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
                    {t(section.translationKey)}
                    <ChevronDown className={cn("transition-transform", open ? "rotate-180" : "")} size={14} aria-hidden="true" />
                  </button>
                )}
                {open || sidebarCollapsed ? (
                  <div className={cn("space-y-1", sidebarCollapsed ? "" : "mt-1")}>
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      const active = item.label === activeItem;
                      const itemLabel = t(item.translationKey);
                      return (
                        <ClientLink
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
                          title={sidebarCollapsed ? itemLabel : undefined}
                        >
                          <Icon size={17} aria-hidden="true" />
                          <span className={cn(sidebarCollapsed ? "sr-only" : "")}>{itemLabel}</span>
                        </ClientLink>
                      );
                    })}
                  </div>
                ) : null}
              </section>
            );
          })}
        </nav>
      </aside>

      {secondarySpaceMenuOpen ? (
        <div
          className={cn(
            "fixed top-[4.5rem] z-50 w-[min(340px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-white px-4 py-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
            sidebarCollapsed ? "left-[84px]" : "left-[292px]",
          )}
        >
          <p className="font-medium">{t("shell:workspace.secondaryLabel")}</p>
          <div className="mt-3 space-y-2">
            {secondarySpaces.length ? (
              secondarySpaces.map((space) => {
                const selected = space.id === selectedSecondarySpace?.id;
                return (
                  <button
                    className={cn(
                      "block w-full rounded-lg border px-3 py-2 text-left transition-colors",
                      selected ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]" : "border-transparent hover:bg-[var(--color-hover-surface)]",
                    )}
                    type="button"
                    aria-pressed={selected}
                    key={space.id}
                    onClick={() => selectSecondarySpace(space.id)}
                  >
                    <span className="block text-sm font-medium">{space.name}</span>
                    <span className="mt-0.5 block text-[12px] text-[var(--color-on-surface-variant)]">{space.description}</span>
                    <span className="mt-1 block text-[11px] text-[var(--color-on-surface-variant)]">
                      {t("shell:workspace.inherited", { count: space.inheritedResources })} · {t("shell:workspace.localOverrides", { count: space.localOverrides })} · {t("shell:workspace.status", { status: space.status })}
                    </span>
                  </button>
                );
              })
            ) : (
              <p className="text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.noSecondarySpaces")}</p>
            )}
          </div>
        </div>
      ) : null}

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
        {t("shell:assistant.title")}
      </button>
      {agentOpen ? (
        <section className="fixed bottom-24 right-6 z-40 w-[320px] rounded-2xl border border-[var(--color-card-border)] bg-white p-4 shadow-[0_18px_50px_rgba(0,0,0,0.14)]">
          <p className="text-sm font-semibold">{t("shell:assistant.title")}</p>
          <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">{t("shell:assistant.body")}</p>
          <button className="mt-3 h-8 rounded-lg bg-[var(--color-surface-container)] px-3 text-[13px] font-medium hover:bg-[var(--color-hover-surface)]" type="button" onClick={() => setAgentOpen(false)}>
            {t("common:actions.close")}
          </button>
        </section>
      ) : null}
    </div>
    </WorkspaceSpacesContext.Provider>
    </TronShellFrameContext.Provider>
  );
}
