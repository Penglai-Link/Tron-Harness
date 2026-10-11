// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { RuntimePanel } from "./RuntimePanel";
import { observability } from "@/i18n/locales/zh-CN/observability";
vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: (key: string) => observability[key as keyof typeof observability] ?? key }) }));
vi.mock("@/shared", () => ({ SurfaceCard: ({ children }: { children: ReactNode }) => <div>{children}</div>, useLocaleFormatters: () => ({ formatNumber: String, formatDateTime: String }) }));
vi.mock("../hooks/useObservability", () => ({
  useRuntimeMetrics: () => ({ data: null }),
  useMetricPoints: () => ({ data: { total: 4, records: [
    { pointId: "cpu", name: "process.cpu.utilization", description: "", kind: "Gauge", unit: "1", timestamp: 1, data: { asDouble: 0.0369 } },
    { pointId: "memory", name: "process.memory.usage", description: "", kind: "Gauge", unit: "By", timestamp: 1, data: { asInt: "387694592" } },
    { pointId: "custom", name: "custom", description: "Producer description", kind: "Sum", unit: "{request}", timestamp: 1, data: { asInt: "2" } },
    { pointId: "unknown", name: "unknown", description: "", kind: "Gauge", unit: "1", timestamp: 1, data: { asDouble: 0.5 } },
  ] } }),
}));
afterEach(cleanup);
it("explains built-in metrics and units while retaining producer metadata and original values", () => {
  render(<RuntimePanel />);
  expect(screen.getByText("指标描述")).toBeTruthy();
  expect(screen.getByText(observability.cpuDescription)).toBeTruthy();
  expect(screen.getByText(observability.memoryDescription)).toBeTruthy();
  expect(screen.getByText("比例（1 = 100%）")).toBeTruthy();
  expect(screen.getByText("无量纲（1）")).toBeTruthy();
  expect(screen.getByText("字节（By）")).toBeTruthy();
  expect(screen.getByText("0.0369")).toBeTruthy();
  expect(screen.getByText("Producer description")).toBeTruthy();
  expect(screen.getByText("采集方未提供描述")).toBeTruthy();
  expect(screen.getByText("{request}")).toBeTruthy();
  expect(screen.getByRole("button", { name: "指标类型说明" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "指标单位说明" })).toBeTruthy();
});
