// @vitest-environment jsdom
import { useState } from "react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Ellipsis, Folder } from "lucide-react";
import { SidebarProvider } from "@/components/ui/sidebar";
import type { TronNavItem, TronNavSection } from "@/shared/types/ShellNavigation";
import { TronSidebar } from "./TronSidebar";

vi.mock("./chat", () => ({ ChatNavigationWidget: ({ menu }: { menu: { label: string } }) => {
  const [count, setCount] = useState(0);
  return <button onClick={() => setCount(count + 1)}>{menu.label} {count}</button>;
} }));
beforeEach(() => vi.stubGlobal("matchMedia", () => ({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() })));
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
const page: TronNavItem = { menuCode: "TASKS", label: "任务", href: "/tasks", icon: Folder, translationKey: "" };
const widget: TronNavItem = { menuCode: "PROJECTS", label: "项目", href: "", icon: Folder, widgetCode: "chat.projects", translationKey: "" };

function Harness({ navigation, compact = false }: { navigation: TronNavSection[]; compact?: boolean }) {
  const [openNavigationItems, setOpenNavigationItems] = useState<Record<string, boolean>>({});
  const location = useLocation();
  return <SidebarProvider defaultOpen={!compact}>
    <TronSidebar activeItem="" navigation={navigation} onNavigate={() => {}} onToggleSection={() => {}}
      openNavigationItems={openNavigationItems} setOpenNavigationItems={setOpenNavigationItems} openSections={{}}
      renderDomainSelector={() => null} renderFooter={() => null} translate={(key) => key} />
    <output>{location.pathname}</output>
  </SidebarProvider>;
}

it.each([false, true])("opens Explore more with interactive widgets and closes after page navigation (compact=%s)", async (compact) => {
  render(<MemoryRouter><Harness compact={compact} navigation={[{ id: "EXPLORE_MORE", label: "探索更多", icon: Ellipsis, flyout: true, translationKey: "", items: [widget, page] }]} /></MemoryRouter>);
  fireEvent.click(screen.getByRole("button", { name: "探索更多" }));
  fireEvent.click(await screen.findByRole("button", { name: "项目 0" }));
  expect(screen.getByRole("button", { name: "项目 1" })).toBeTruthy();
  fireEvent.click(screen.getByRole("link", { name: "任务" }));
  expect(screen.getByText("/tasks")).toBeTruthy();
  await waitFor(() => expect(screen.queryByRole("button", { name: "项目 1" })).toBeNull());
});

it("collapses an inline group on the first click and opens a widget's own flyout", async () => {
  render(<MemoryRouter><Harness navigation={[{ label: "导航", translationKey: "", items: [
    { ...page, label: "分组", isGroup: true, children: [page], childDisplay: "Inline" },
    { ...widget, childDisplay: "Flyout" },
  ] }]} /></MemoryRouter>);
  expect(screen.getByRole("link", { name: "任务" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "shell:aria.collapseNavigationItem" }));
  expect(screen.queryByRole("link", { name: "任务" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "项目" }));
  expect(await screen.findByRole("button", { name: "项目 0" })).toBeTruthy();
});
