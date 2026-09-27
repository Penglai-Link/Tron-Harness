// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ToastProvider } from "@/components/ui/toast";
import { useServiceQueryState } from "@/shared/api/serviceConfig";

vi.mock("@/i18n", () => ({
  fallbackLocale: "en",
  applyCurrentLocaleHeader: (headers: Headers) => headers.set("Accept-Language", "en-US"),
  i18n: {
    language: "en",
    resolvedLanguage: "en",
  },
}));

interface QueryRecord {
  label: string;
}

function jsonResponse(data: QueryRecord[]) {
  return Promise.resolve(new Response(JSON.stringify({
    code: "SUCCESS",
    data,
    message: "ok",
  }), { status: 200 }));
}

function deferredResponse(data: QueryRecord[]) {
  let resolve!: () => void;
  const gate = new Promise<void>((done) => {
    resolve = done;
  });
  return {
    promise: gate.then(() => jsonResponse(data)),
    resolve,
  };
}

function QueryHarness() {
  const [refreshKey, setRefreshKey] = useState(0);
  const state = useServiceQueryState<QueryRecord[]>({
    apiInitialData: () => [],
    deps: [],
    domain: "chat",
    endpoint: "GET /api/test-records",
    mockData: () => [{ label: "mock" }],
    refreshKey,
  });

  return (
    <>
      <button type="button" onClick={() => setRefreshKey((current) => current + 1)}>
        refresh
      </button>
      <p data-testid="records">
        {state.data.map((record) => record.label).join(",") || "empty"}
      </p>
      <p data-testid="loading">{String(state.loading)}</p>
    </>
  );
}

describe("useServiceQueryState", () => {
  beforeEach(() => {
    vi.stubEnv("VITE_TRON_DATA_SOURCE", "api");
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
  });

  it("keeps API data visible while refreshKey refetches", async () => {
    const refresh = deferredResponse([{ label: "fresh" }]);
    const fetchMock = vi.mocked(fetch);
    fetchMock
      .mockImplementationOnce(() => jsonResponse([{ label: "stale" }]))
      .mockImplementationOnce(() => refresh.promise);

    render(
      <ToastProvider>
        <QueryHarness />
      </ToastProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("records").textContent).toBe("stale");
    });

    fireEvent.click(screen.getByRole("button", { name: "refresh" }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(2);
    });
    expect(screen.getByTestId("records").textContent).toBe("stale");
    expect(screen.getByTestId("loading").textContent).toBe("true");

    refresh.resolve();

    await waitFor(() => {
      expect(screen.getByTestId("records").textContent).toBe("fresh");
    });
  });
});
