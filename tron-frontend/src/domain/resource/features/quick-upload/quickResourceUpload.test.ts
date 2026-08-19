// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";

import { useQuickResourceUploadHandoffStore } from "@/domain/resource/model";

import {
  evaluateQuickResourceUpload,
  generateQuickResourceCode,
  imageFormatForFileName,
  quickResourceMaxFileSizeBytes,
  suggestQuickResourceType,
} from "./quickResourceUpload";

describe("quick resource upload helpers", () => {
  it("suggests a built-in type from the file suffix", () => {
    expect(suggestQuickResourceType("release.pdf")).toBe("PDF");
    expect(suggestQuickResourceType("brand.SVG")).toBe("IMAGE");
    expect(suggestQuickResourceType("archive.tar.gz")).toBe("FILE");
    expect(imageFormatForFileName("photo.jpeg")).toBe("JPEG");
  });

  it("creates a valid, visible ASCII resource code without hiding a collision suffix", () => {
    expect(generateQuickResourceCode("Quarterly Report (final).pdf", 123)).toBe("quarterly-report-final");
    expect(generateQuickResourceCode("中文.pdf", 123)).toBe("FILE_123");
    expect(generateQuickResourceCode(`${"a".repeat(90)}.txt`, 123)).toHaveLength(64);
  });

  it("marks complete uploads as ready", () => {
    const file = new File(["hello"], "readme.md");
    expect(evaluateQuickResourceUpload({
      binaryName: file.name,
      file,
      type: "FILE",
    })).toEqual({ binaryName: "readme.md", kind: "READY", type: "FILE" });
  });

  it("accepts PDFs whose page count cannot be read", () => {
    const file = new File(["pdf"], "policy.pdf");
    expect(evaluateQuickResourceUpload({
      binaryName: file.name,
      file,
      pageCount: 0,
      type: "PDF",
    })).toEqual({ binaryName: "policy.pdf", kind: "READY", type: "PDF" });
  });

  it("blocks files above the fixed size limit", () => {
    const file = { name: "archive.zip", size: quickResourceMaxFileSizeBytes + 1 };
    expect(evaluateQuickResourceUpload({
      binaryName: file.name,
      file,
      type: "FILE",
    })).toEqual({ kind: "BLOCKED", reason: "FILE_TOO_LARGE" });
  });
});

describe("quick upload handoff", () => {
  beforeEach(() => {
    useQuickResourceUploadHandoffStore.setState({ records: {} });
  });

  it("keeps the File only in memory and clears it after the editor consumes the handoff", () => {
    const file = new File(["asset"], "asset.txt");
    const id = useQuickResourceUploadHandoffStore.getState().put({
      allowInheritance: true,
      binaryName: "asset.txt",
      description: "",
      file,
      kind: "FILE",
      name: "asset",
      pageCount: null,
      resourceCode: "asset",
      workspaceCode: "TRON_CORE",
    });

    expect(useQuickResourceUploadHandoffStore.getState().peek(id)?.file).toBe(file);
    useQuickResourceUploadHandoffStore.getState().clear(id);
    expect(useQuickResourceUploadHandoffStore.getState().peek(id)).toBeNull();
  });
});
