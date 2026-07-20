// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

import { readPdfPageCount } from "./pdfPageCount";

describe("readPdfPageCount", () => {
  it("returns the browser PDF.js page count and releases both document handles", async () => {
    const destroyDocument = vi.fn().mockResolvedValue(undefined);
    const destroyTask = vi.fn().mockResolvedValue(undefined);
    const loader = vi.fn().mockResolvedValue({
      GlobalWorkerOptions: { workerSrc: "" },
      getDocument: vi.fn().mockReturnValue({
        destroy: destroyTask,
        promise: Promise.resolve({ destroy: destroyDocument, numPages: 12 }),
      }),
    });
    const file = {
      arrayBuffer: vi.fn().mockResolvedValue(new Uint8Array([37, 80, 68, 70]).buffer),
    } as unknown as File;

    await expect(readPdfPageCount(file, loader)).resolves.toBe(12);
    expect(destroyDocument).toHaveBeenCalledOnce();
    expect(destroyTask).toHaveBeenCalledOnce();
  });

  it("propagates a reader failure so the caller can hand off pageCount 0 to the editor", async () => {
    const loader = vi.fn().mockResolvedValue({
      GlobalWorkerOptions: { workerSrc: "" },
      getDocument: vi.fn().mockReturnValue({
        destroy: vi.fn().mockResolvedValue(undefined),
        promise: Promise.reject(new Error("invalid PDF")),
      }),
    });
    const file = { arrayBuffer: vi.fn().mockResolvedValue(new ArrayBuffer(0)) } as unknown as File;

    await expect(readPdfPageCount(file, loader)).rejects.toThrow("invalid PDF");
  });
});
