// @vitest-environment jsdom
import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pages as enPages } from "@/i18n/locales/en-US/pages";
import { pages as zhPages } from "@/i18n/locales/zh-CN/pages";
const locale = vi.hoisted(() => ({ language: "en-US" }));
vi.mock("@/domain/agent/api", () => ({ fetchToolDomains: vi.fn(async () => []), listTools: vi.fn(async () => []) }));
vi.mock("@/domain/agent/mock", () => ({ listToolDomainsMock: () => [], listToolsMock: () => [] }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({
  i18n: { language: locale.language },
  t: (key: string) => key.split(".").reduce((value: unknown, item) => (value as Record<string, unknown>)[item], locale.language === "zh-CN" ? zhPages : enPages),
}) }));
import { useChatWorkRailLabels } from "./useChatWorkRailLabels";
beforeEach(() => { locale.language = "en-US"; });
afterEach(cleanup);
describe("useChatWorkRailLabels stopped copy", () => {
  it("passes the selected locale's stopped label without changing failure labels", async () => {
    const { result, rerender } = renderHook(() => useChatWorkRailLabels());
    await waitFor(() => expect(result.current.stopped).toBe("Run stopped"));
    expect(result.current.failed).toBe("Run failed");
    locale.language = "zh-CN";
    rerender();
    await waitFor(() => expect(result.current.stopped).toBe("已停止"));
    expect(result.current.failed).toBe("运行失败");
  });
});
