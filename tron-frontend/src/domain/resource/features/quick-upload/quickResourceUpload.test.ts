// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";

import type { ResourceDefinitionVersionDetail } from "@/domain/resource/model";
import { useQuickResourceUploadHandoffStore } from "@/domain/resource/model";

import {
  buildQuickResourceAttributes,
  evaluateQuickResourceUpload,
  generateQuickResourceCode,
  imageFormatForFileName,
  suggestQuickResourceType,
} from "./quickResourceUpload";

function definition(
  definitionCode: "FILE" | "IMAGE" | "PDF",
  attributesSchema: Record<string, unknown>,
  binarySchema: Partial<ResourceDefinitionVersionDetail["binarySchema"]> = {},
): ResourceDefinitionVersionDetail {
  const resolvedBinarySchema = {
    maxCount: 1,
    maxNameLength: 255,
    maxSizeBytes: 1024,
    minCount: 1,
    namePattern: "^[^/]+$",
    storageProfile: "@default",
    ...binarySchema,
  };
  return {
    allowInheritance: true,
    attributesSchema: attributesSchema as ResourceDefinitionVersionDetail["attributesSchema"],
    binarySchema: resolvedBinarySchema,
    effectiveBinaryPolicy: {
      maxCount: resolvedBinarySchema.maxCount,
      maxNameLength: resolvedBinarySchema.maxNameLength,
      maxSizeBytes: resolvedBinarySchema.maxSizeBytes,
      minCount: resolvedBinarySchema.minCount,
      namePattern: resolvedBinarySchema.namePattern || null,
    },
    createdAt: "2026-07-20T00:00:00.000Z",
    definitionCode,
    definitionVersion: "v1-A1b",
    description: "",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "ACTIVE",
    name: definitionCode,
    ownerWorkspaceCode: null,
    ownerWorkspaceName: null,
    preset: true,
    publishedAt: "2026-07-20T00:00:00.000Z",
    scopeKind: "PLATFORM",
    sourceKind: "PLATFORM",
    updatedAt: "2026-07-20T00:00:00.000Z",
    validationIssueCount: 0,
    validationStatus: "VALID",
  };
}

const fileSchema = {
  additionalProperties: false,
  properties: {
    fileExtension: { type: "string" },
    fileName: { type: "string" },
    sizeBytes: { minimum: 0, type: "integer" },
  },
  required: ["fileName", "fileExtension", "sizeBytes"],
  type: "object",
};

const pdfSchema = {
  additionalProperties: false,
  properties: {
    fileName: { type: "string" },
    pageCount: { minimum: 1, type: "integer" },
    sizeBytes: { minimum: 0, type: "integer" },
  },
  required: ["fileName", "pageCount", "sizeBytes"],
  type: "object",
};

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

  it("prefills caller-owned file and image metadata", () => {
    const textFile = new File(["hello"], "readme.md");
    expect(buildQuickResourceAttributes({ binaryName: "readme.md", file: textFile, type: "FILE" })).toEqual({
      fileExtension: "md",
      fileName: "readme.md",
      sizeBytes: 5,
    });

    const image = new File(["png"], "product.png");
    expect(buildQuickResourceAttributes({ binaryName: "product.png", file: image, type: "IMAGE" })).toEqual({
      images: [{
        binaryName: "product.png",
        fileExtension: "png",
        fileName: "product.png",
        imageFormat: "PNG",
        sizeBytes: 3,
      }],
    });
  });

  it("allows direct creation only when the effective Schema and binary policy both pass", () => {
    const file = new File(["hello"], "readme.md");
    const result = evaluateQuickResourceUpload({
      binaryName: file.name,
      definition: definition("FILE", fileSchema),
      file,
      type: "FILE",
    });

    expect(result).toMatchObject({ kind: "READY", attributes: { fileName: "readme.md", sizeBytes: 5 } });
  });

  it("hands off repairable Schema, page-count, name, and file-count cases to the editor", () => {
    const file = new File(["pdf"], "policy.pdf");
    const pdf = evaluateQuickResourceUpload({
      binaryName: file.name,
      definition: definition("PDF", pdfSchema),
      file,
      pageCount: 0,
      type: "PDF",
    });
    expect(pdf).toMatchObject({
      attributes: { pageCount: 0 },
      kind: "EDITOR",
      reasons: expect.arrayContaining(["PDF_PAGE_COUNT"]),
    });

    const invalidName = evaluateQuickResourceUpload({
      binaryName: "not allowed.pdf",
      definition: definition("FILE", fileSchema, { namePattern: "^[a-z]+$" }),
      file,
      type: "FILE",
    });
    expect(invalidName).toMatchObject({ kind: "EDITOR", reasons: ["BINARY_NAME"] });

    const needsMoreFiles = evaluateQuickResourceUpload({
      binaryName: file.name,
      definition: definition("FILE", fileSchema, { maxCount: 2, minCount: 2 }),
      file,
      type: "FILE",
    });
    expect(needsMoreFiles).toMatchObject({ kind: "EDITOR", reasons: ["REQUIRES_MORE_FILES"] });
  });

  it("keeps non-repairable definition and size failures in the dialog", () => {
    const file = new File(["too large"], "archive.zip");
    const resolvedLimit = definition("FILE", fileSchema, { maxSizeBytes: 0 });
    resolvedLimit.effectiveBinaryPolicy = {
      ...resolvedLimit.effectiveBinaryPolicy!,
      maxSizeBytes: 1,
    };
    expect(evaluateQuickResourceUpload({
      binaryName: file.name,
      definition: resolvedLimit,
      file,
      type: "FILE",
    })).toEqual({ kind: "BLOCKED", reason: "FILE_TOO_LARGE" });

    expect(evaluateQuickResourceUpload({
      binaryName: file.name,
      definition: null,
      file,
      type: "FILE",
    })).toEqual({ kind: "BLOCKED", reason: "DEFINITION_UNAVAILABLE" });
  });

  it("does not reject an old detail response just because its size limit was omitted", () => {
    const file = new File(["image-bytes"], "logo.png");
    const legacyDefinition = definition("FILE", fileSchema, { maxSizeBytes: 0 });
    legacyDefinition.effectiveBinaryPolicy = null;

    expect(evaluateQuickResourceUpload({
      binaryName: file.name,
      definition: legacyDefinition,
      file,
      type: "FILE",
    })).toMatchObject({ kind: "READY" });
  });
});

describe("quick upload handoff", () => {
  beforeEach(() => {
    useQuickResourceUploadHandoffStore.setState({ records: {} });
  });

  it("keeps the File only in memory and clears it after the editor consumes the handoff", () => {
    const file = new File(["asset"], "asset.txt");
    const item = definition("FILE", fileSchema);
    const id = useQuickResourceUploadHandoffStore.getState().put({
      allowInheritance: true,
      attributes: { fileExtension: "txt", fileName: "asset.txt", sizeBytes: 5 },
      binaryName: "asset.txt",
      definition: item,
      description: "",
      file,
      name: "asset",
      resourceCode: "asset",
      scopeCode: "TRON_CORE",
      scopeKind: "WORKSPACE",
    });

    expect(useQuickResourceUploadHandoffStore.getState().peek(id)?.file).toBe(file);
    useQuickResourceUploadHandoffStore.getState().clear(id);
    expect(useQuickResourceUploadHandoffStore.getState().peek(id)).toBeNull();
  });
});
