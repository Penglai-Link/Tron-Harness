// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useRagGraph } from "./useRagGraph";
import { defaultRagGraphOptions, type RagGraphWindow } from "../model/RagGraphModel";
const api = vi.hoisted(() => ({ browse: vi.fn(), search: vi.fn(), expand: vi.fn() }));
vi.mock("../api/ragGraphApi", () => ({ browseRagGraph: api.browse, searchRagGraph: api.search, expandRagGraph: api.expand }));
const empty = (snapshotId: string): RagGraphWindow => ({ snapshotId, seeds: [], nodes: [], edges: [], references: [], stages: [], nextCursor: null, expiresAt: null, unfinished: [], truncatedBy: [], durationMs: 1 });
const query = { ...defaultRagGraphOptions, query: "Alpha", corpusIds: [], filters: {} };
beforeEach(() => vi.clearAllMocks()); afterEach(cleanup);
it("ignores late results after a corpus change resets the graph", async () => {
  let finish!: (value: RagGraphWindow) => void; api.search.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
  const { result } = renderHook(useRagGraph); let task!: Promise<void>;
  act(() => { task = result.current.request(query, false); });
  act(() => result.current.reset()); expect(api.search.mock.calls[0][1].aborted).toBe(true);
  await act(async () => { finish(empty("stale")); await task; });
  expect(result.current.window).toBeNull(); expect(result.current.loading).toBe(false);
});
it("discards stale content on a rejected continuation and sends cursor alone", async () => {
  api.search.mockResolvedValue(empty("first")); api.expand.mockRejectedValue(new Error("Snapshot changed"));
  const { result } = renderHook(useRagGraph);
  await act(() => result.current.request(query, false)); expect(result.current.window?.snapshotId).toBe("first");
  await act(() => result.current.request({ cursor: "opaque" }, true));
  expect(api.expand.mock.calls[0][0]).toEqual({ cursor: "opaque" });
  expect(result.current.window).toBeNull(); expect(result.current.error?.message).toBe("Snapshot changed");
});
it("ignores an aborted earlier search when the newer one finishes first", async () => {
  let finish!: (value: RagGraphWindow) => void;
  api.search.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; })).mockResolvedValueOnce(empty("new"));
  const { result } = renderHook(useRagGraph); let old!: Promise<void>;
  act(() => { old = result.current.request(query, false); });
  await act(() => result.current.request({ ...query, query: "Beta" }, false));
  await act(async () => { finish(empty("old")); await old; });
  expect(result.current.window?.snapshotId).toBe("new");
});

it("aborts a stale overview when a new corpus is selected", async () => {
  let finish!: (value: RagGraphWindow) => void;
  api.browse.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; })).mockResolvedValueOnce(empty("new"));
  const { result } = renderHook(useRagGraph); let old!: Promise<void>;
  const browse = { ...defaultRagGraphOptions, corpusIds: [], filters: {} };
  act(() => { old = result.current.request(browse, false); });
  await act(() => result.current.request({ ...browse, corpusIds: ["another"] }, false));
  expect(api.browse.mock.calls[0][1].aborted).toBe(true);
  await act(async () => { finish(empty("old")); await old; });
  expect(result.current.window?.snapshotId).toBe("new");
  expect(api.search).not.toHaveBeenCalled();
});
