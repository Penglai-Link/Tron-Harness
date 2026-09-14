// @vitest-environment jsdom
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { FileSystemClient } from "@/shared/fs/electronFs";
import type { FsEntry } from "@/shared/fs/types";

import { joinPath, parentPath, useFsTree } from "./useFsTree";

function dir(name: string): FsEntry {
  return { name, isDir: true, isFile: false, isSymlink: false, size: 0 };
}

function file(name: string): FsEntry {
  return { name, isDir: false, isFile: true, isSymlink: false, size: 1 };
}

function makeClient(entries: Record<string, FsEntry[]>): FileSystemClient {
  return {
    pickRoot: vi.fn(async () => ({ rootId: "r1", name: "root", path: "/root" })),
    list: vi.fn(async (_rootId: string, path: string) => entries[path] ?? []),
  } as unknown as FileSystemClient;
}

describe("path helpers", () => {
  it("joinPath and parentPath round-trip", () => {
    expect(joinPath("src", "a.ts")).toBe("src/a.ts");
    expect(joinPath("", "src")).toBe("src");
    expect(parentPath("src/a.ts")).toBe("src");
    expect(parentPath("a.ts")).toBe("");
  });
});

describe("useFsTree", () => {
  it("loads root entries after pickRoot", async () => {
    const client = makeClient({ "": [dir("src"), file("README.md")] });
    const { result } = renderHook(() => useFsTree({ client }));

    let info;
    await act(async () => {
      info = await result.current.pickRoot();
    });

    expect(info).toMatchObject({ rootId: "r1" });
    expect(result.current.tree.get("")?.map((entry) => entry.name)).toEqual(["src", "README.md"]);
  });

  it("lazily loads a directory when it is expanded", async () => {
    const client = makeClient({
      "": [dir("src")],
      src: [file("a.ts")],
    });
    const { result } = renderHook(() => useFsTree({ client }));

    await act(async () => {
      await result.current.pickRoot();
    });

    expect(result.current.tree.get("src")).toBeUndefined();

    act(() => {
      result.current.toggleDir("src");
    });

    expect(result.current.expanded.has("src")).toBe(true);
    await waitFor(() => expect(result.current.tree.get("src")).toBeDefined());
    expect(result.current.tree.get("src")?.map((entry) => entry.name)).toEqual(["a.ts"]);
  });

  it("selectEntry updates the selected path", async () => {
    const client = makeClient({ "": [file("a.ts")] });
    const { result } = renderHook(() => useFsTree({ client }));

    await act(async () => {
      await result.current.pickRoot();
    });

    act(() => {
      result.current.selectEntry("a.ts");
    });

    expect(result.current.selectedPath).toBe("a.ts");
  });
});
