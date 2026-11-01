// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "@/domain/agent/model";
import { InternalAgentToolSelector } from "./InternalAgentToolSelector";

vi.mock("react-i18next", () => ({ useTranslation: () => ({
  i18n: { language: "zh-CN", resolvedLanguage: "zh-CN" },
  t: (_key: string, options?: { defaultValue?: string; count?: number }) => (options?.defaultValue ?? _key).replace("{{count}}", String(options?.count ?? "")),
}) }));
vi.mock("./InternalAgentSkillField", () => ({ DisabledResourceReason: () => null }));
afterEach(cleanup);

it("filters tools by category without appending category labels to domain names", () => {
  const domain: ToolCatalogDomainRecord = { domainCode: "shared", name: "共享领域", description: "", toolCount: 2, sampleTools: [], hasMutationTools: true };
  const tools = [
    { toolCode: "first", toolName: "first", name: "First", description: "first", domainCode: "shared", operationKind: "query", category: { code: "platform", names: { "zh-CN": "平台工具" } } },
    { toolCode: "second", toolName: "second", name: "Second", description: "second", domainCode: "shared", operationKind: "command", category: { code: "system", names: { "zh-CN": "系统工具" } } },
  ] as PlatformToolRecord[];
  const toggle = vi.fn();
  render(<InternalAgentToolSelector tools={tools} toolDomains={[domain]} selectedToolCodes={["first"]} onToggleTool={toggle} />);
  expect(screen.getAllByRole("button", { name: /共享领域/ })).toHaveLength(2);
  expect(screen.queryByText("共享领域 · 平台工具")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "系统工具 (1)" }));
  fireEvent.click(screen.getByRole("button", { name: /共享领域/ }));
  fireEvent.click(screen.getByRole("checkbox"));
  expect(toggle).toHaveBeenCalledWith("second", true);
  fireEvent.click(screen.getByRole("button", { name: "Read-only tools (1)" }));
  fireEvent.click(screen.getByRole("button", { name: /共享领域/ }));
  expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
});
