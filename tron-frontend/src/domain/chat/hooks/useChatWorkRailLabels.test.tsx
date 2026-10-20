// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useChatWorkRailLabels } from "./useChatWorkRailLabels";

vi.mock("@/domain/agent/api", () => ({ fetchToolDomains: vi.fn(), listTools: vi.fn() }));
vi.mock("@/domain/agent/mock", () => ({ listToolDomainsMock: () => [], listToolsMock: () => [] }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "mock" }));

function SummaryProbe() {
  const labels = useChatWorkRailLabels();
  const text = labels.tool?.groupSummary?.({
    commands: 2, editedFiles: 2, editOperations: 0, reads: 0, searches: 0, otherActions: 0,
  });
  return <span>{text}</span>;
}

afterEach(cleanup);

async function setLocale(language: string) {
  const values = new Map<string, string>();
  Object.defineProperty(window, "localStorage", { configurable: true, value: {
    get length() { return values.size; },
    key: (index: number) => [...values.keys()][index] ?? null,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, String(value)); },
    removeItem: (key: string) => { values.delete(key); },
    clear: () => { values.clear(); },
  } });
  const { i18n } = await import("@/i18n");
  await i18n.changeLanguage(language);
}

it("formats action summaries in Chinese", async () => {
  await setLocale("zh-CN");
  render(<SummaryProbe />);
  expect(await screen.findByText("执行 2 个命令，编辑 2 个文件")).toBeTruthy();
});

it("formats action summaries in English", async () => {
  await setLocale("en-US");
  render(<SummaryProbe />);
  expect(await screen.findByText(/Executed 2 commands/)).toBeTruthy();
  expect(screen.getByText(/Edited 2 files/)).toBeTruthy();
});
