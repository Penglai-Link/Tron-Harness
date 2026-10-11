// @vitest-environment jsdom
import { cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useApplicationLogs, useContinuousLogs } from "./useObservability";
const state = vi.hoisted(() => ({ isError: false, error: vi.fn() }));
vi.mock("@tanstack/react-query", () => ({ useQuery: () => ({ isError: state.isError }), useInfiniteQuery: () => ({ isError: state.isError }) }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: state.error }) }));
vi.mock("@/shared/hooks/useWorkspaceSpaces", () => ({ useWorkspaceSpaces: () => ({}) }));
vi.mock("../api/observabilityApi", () => ({}));
afterEach(() => { cleanup(); state.isError = false; vi.clearAllMocks(); });
const query = { date: "2026-10-11", timeZone: "Asia/Shanghai", keyword: "", traceId: "", spanId: "", level: "" as const, current: 1, size: 15 };
for (const [name, hook] of [["blocks", () => useApplicationLogs(query, true)], ["continuous", () => useContinuousLogs(query, true)]] as const) {
  it(`reports ${name} failures as a toast once until recovery`, () => {
    state.isError = true;
    const { rerender } = renderHook(() => { hook(); return null; });
    expect(state.error).toHaveBeenCalledWith("logsLoadFailed");
    rerender();
    expect(state.error).toHaveBeenCalledTimes(1);
    state.isError = false;
    rerender();
    state.isError = true;
    rerender();
    expect(state.error).toHaveBeenCalledTimes(2);
  });
}
