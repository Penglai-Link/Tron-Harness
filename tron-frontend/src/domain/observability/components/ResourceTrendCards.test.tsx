// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { PluginCountCard } from "./ResourceTrendCards";

const api = vi.hoisted(() => ({
  plugins: vi.fn(),
  tools: vi.fn(),
  skills: vi.fn(),
  internalAgents: vi.fn(),
  systemAgents: vi.fn(),
  dudes: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => ({
        pluginCount: "Plugins",
        enabledPluginCount: "Enabled plugins",
        toolCount: "Tools",
        skillCount: "Skills",
        agentCount: "Agents",
        dudeCount: "Dudes",
      }[key] ?? key),
  }),
}));
vi.mock("@/shared", () => ({
  SurfaceCard: ({ children }: { children: ReactNode }) => <section>{children}</section>,
  useLocaleFormatters: () => ({ formatNumber: (value: number) => String(value) }),
}));
vi.mock("@/domain/settings/api/capabilityApi", () => ({ listCapabilityPacks: api.plugins }));
vi.mock("@/domain/agent/api/agentManagementApi", () => ({
  fetchPlatformSystemAgents: api.systemAgents,
  fetchWorkspaceInternalAgents: api.internalAgents,
  listTools: api.tools,
}));
vi.mock("@/domain/skill/api", () => ({ fetchRegistrySkills: api.skills }));
vi.mock("@/domain/dude/api/dudeApi", () => ({ listDudes: api.dudes }));
vi.mock("../api/observabilityApi", () => ({ queryMetricPoints: vi.fn().mockResolvedValue({ records: [] }) }));

beforeEach(() => {
  api.plugins.mockResolvedValue([
    { effective: true, contents: [{ kind: "Plugin", name: "plugin-a" }] },
    { effective: false, contents: [{ kind: "Plugin", name: "plugin-a" }, { kind: "Plugin", name: "plugin-b" }] },
  ]);
  api.tools.mockResolvedValue([{}, {}, {}]);
  api.skills.mockResolvedValue([{}, {}]);
  api.internalAgents.mockResolvedValue([{}, {}]);
  api.systemAgents.mockResolvedValue([{}]);
  api.dudes.mockResolvedValue([{}, {}, {}, {}]);
});
afterEach(cleanup);

it("shows platform inventory counts alongside plugin count", async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><PluginCountCard /></QueryClientProvider>);

  await waitFor(() => {
    expect(screen.getByText("Tools")).toBeTruthy();
    expect(screen.getByText("Skills")).toBeTruthy();
    expect(screen.getByText("Agents")).toBeTruthy();
    expect(screen.getByText("Dudes")).toBeTruthy();
    expect(screen.getByText("4")).toBeTruthy();
  });
  expect(screen.getAllByText("3")).toHaveLength(2);
  expect(screen.getByText("Plugins").nextElementSibling?.textContent).toBe("2");
  expect(screen.getByText("Enabled plugins").nextElementSibling?.textContent).toBe("1");
  expect(screen.getByText("4")).toBeTruthy();
  expect(document.querySelectorAll("section")).toHaveLength(1);
});
