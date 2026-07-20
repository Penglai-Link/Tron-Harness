// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  adaptResourceDefinitionVersionDetail,
  adaptResourceVersionDetail,
} from "@/domain/resource/adapters";
import {
  createResource,
  deleteResource,
  deleteResourceDefinition,
  downloadResourceBinary,
  pageResourceDefinitions,
  setResourceDefinitionEnabled,
  updateResource,
  uploadResourceBinary,
} from "@/domain/resource/api";
import { setApiAccessToken } from "@/shared/api/httpClient";

const definitionPageRequest = {
  current: 1,
  definitionCode: null,
  definitionStatus: null,
  lifecycleStatus: "ACTIVE" as const,
  scopeKind: "WORKSPACE" as const,
  search: null,
  size: 20,
  sourceKind: null,
  validationStatus: "VALID" as const,
  workspaceCode: "TRON_CORE",
};

function jsonResponse(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
    status,
  });
}

afterEach(() => {
  setApiAccessToken(null);
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("Resource API client", () => {
  it("adapts the management projection without leaking wire details into the page model", () => {
    const definition = adaptResourceDefinitionVersionDetail({
      allowInheritance: true,
      attributesSchema: { properties: { title: { type: "string" } }, type: "object" },
      binarySchema: { maxCount: 10, minCount: 1 },
      binaryStorageProfile: "local-default",
      builtIn: false,
      createdAt: 1_767_225_600_000,
      definitionCode: "DOCUMENT",
      definitionStatus: "DISABLED",
      definitionVersion: "v2-A1b",
      description: "Document resource",
      draftRevision: "12",
      effectiveBinaryPolicy: {
        maxCount: 10,
        maxNameLength: 255,
        maxSizeBytes: 1_073_741_824,
        minCount: 1,
        namePattern: null,
      },
      lifecycleStatus: "DRAFT",
      name: "Document",
      publishedAt: null,
      scopeCode: "PLATFORM",
      scopeKind: "PLATFORM",
      sourceKind: "PLATFORM",
      updatedAt: 1_767_312_000_000,
      validationStatus: "VALID",
    });
    const resource = adaptResourceVersionDetail({
      allowInheritance: false,
      attributes: { title: "Quarterly report" },
      attributesSchema: { properties: { title: { type: "string" } }, type: "object" },
      binaries: [{ binaryName: "report.pdf", byteSize: 184_320 }],
      binarySchema: { maxCount: 1 },
      binaryStorageProfile: "local-resource-profile",
      createdAt: 1_767_225_600_000,
      definitionCode: "DOCUMENT",
      definitionName: "Document",
      definitionScopeCode: "TRON_CORE",
      definitionScopeKind: "WORKSPACE",
      definitionStatus: "ENABLED",
      definitionVersion: "v2-A1b",
      description: "Quarterly report",
      draftRevision: "13",
      lifecycleStatus: "DRAFT",
      name: "Q3 report",
      publishedAt: null,
      resourceCode: "q3-report",
      resourceVersion: "v5-C3d",
      scopeCode: "TRON_CORE",
      scopeKind: "WORKSPACE",
      sourceKind: "LOCAL",
      updatedAt: 1_767_312_000_000,
      validationStatus: "VALID",
    });

    expect(definition).toMatchObject({
      binarySchema: { maxCount: 10, maxSizeBytes: 0, storageProfile: "local-default" },
      draftRevision: 12,
      effectiveBinaryPolicy: { maxCount: 10, maxSizeBytes: 1_073_741_824, namePattern: null },
      enabled: false,
      preset: false,
      updatedAt: new Date(1_767_312_000_000).toISOString(),
    });
    expect(resource).toMatchObject({
      attributes: { title: "Quarterly report" },
      binaries: [{ binaryName: "report.pdf", byteSize: 184_320 }],
      binarySchema: { maxCount: 1, maxNameLength: 255, maxSizeBytes: 0, minCount: 0, namePattern: "", storageProfile: "local-resource-profile" },
      draftRevision: 13,
      resourceVersion: "v5-C3d",
    });
  });

  it("sends the page request as authenticated JSON and surfaces management errors", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { current: 1, records: [], size: 20, total: 0 },
        message: "ok",
      }))
      .mockResolvedValueOnce(jsonResponse({
        code: "-1",
        data: {},
        message: "Resource state conflicts with this operation",
      }, 409));
    vi.stubGlobal("fetch", fetchMock);
    setApiAccessToken("resource-test-token");

    await expect(pageResourceDefinitions(definitionPageRequest)).resolves.toEqual({
      current: 1,
      records: [],
      size: 20,
      total: 0,
    });

    const [, request] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(request.method).toBe("POST");
    expect(request.body).toBe(JSON.stringify(definitionPageRequest));
    const headers = new Headers(request.headers);
    expect(headers.get("Authorization")).toBe("Bearer resource-test-token");
    expect(headers.get("Content-Type")).toBe("application/json");

    await expect(pageResourceDefinitions(definitionPageRequest)).rejects.toMatchObject({
      code: "-1",
      message: "Resource state conflicts with this operation",
      status: 409,
    });
  });

  it("uses FormData for upload and returns raw binary downloads as blobs", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: {
          action: "UPLOADED",
          binaryName: "report.pdf",
          byteSize: 7,
          draftRevision: "14",
          resourceCode: "q3-report",
          resourceVersion: "v5-C3d",
          validationErrors: [],
          validationStatus: "VALID",
        },
        message: "ok",
      }))
      .mockResolvedValueOnce(new Response(new Uint8Array([112, 97, 121, 108, 111, 97, 100]), {
        headers: {
          "Content-Disposition": "attachment; filename*=UTF-8''report.pdf",
          "Content-Type": "application/octet-stream",
        },
        status: 200,
      }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(uploadResourceBinary({
      binaryName: "report.pdf",
      expectedDraftRevision: "13",
      file: new File(["payload"], "report.pdf", { type: "application/pdf" }),
      resourceCode: "q3-report",
      resourceVersion: "v5-C3d",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    })).resolves.toMatchObject({
      byteSize: 7,
      draftRevision: "14",
    });

    const [, uploadRequest] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(uploadRequest.method).toBe("POST");
    expect(uploadRequest.body).toBeInstanceOf(FormData);
    const uploadForm = uploadRequest.body as FormData;
    expect(uploadForm.get("binaryName")).toBe("report.pdf");
    expect(uploadForm.get("expectedDraftRevision")).toBe("13");
    expect(uploadForm.get("file")).toBeInstanceOf(File);
    expect(new Headers(uploadRequest.headers).has("Content-Type")).toBe(false);

    const download = await downloadResourceBinary({
      binaryName: "report.pdf",
      resourceCode: "q3-report",
      resourceVersion: "v5-C3d",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    });
    // The Vitest DOM Blob shim does not implement Blob#text(), but it does
    // preserve the response byte length. The backend HTTP integration test
    // covers byte-for-byte streaming; this client test verifies that the raw
    // binary response reaches the caller as a Blob.
    expect(download.blob.size).toBe(7);
    expect(download.contentDisposition).toContain("filename*=");
    expect(download.contentType).toBe("application/octet-stream");
  });

  it("commits ACTIVE resources atomically and keeps definition/delete commands as JSON", async () => {
    const activation = {
      activationResult: "COMMITTED" as const,
      definitionCode: "IMAGE",
      definitionScopeCode: "PLATFORM",
      definitionScopeKind: "PLATFORM" as const,
      definitionVersion: "v1-A1a",
      lifecycleStatus: "ACTIVE" as const,
      previousActiveResourceVersion: null,
      resourceCode: "hero-image",
      resourceVersion: "v1-B2c",
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({ code: "0", data: activation, message: "ok" }))
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { ...activation, previousActiveResourceVersion: "v1-B2c", resourceVersion: "v2-C3d" },
        message: "ok",
      }))
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: {
          activationResult: "COMMITTED",
          definitionCode: "IMAGE",
          definitionVersion: "v2-C3d",
          lifecycleStatus: "ACTIVE",
          previousActiveDefinitionVersion: "v1-A1a",
        },
        message: "ok",
      }))
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { deleted: true, resourceCode: "hero-image" },
        message: "ok",
      }))
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { definitionCode: "CUSTOM_IMAGE", deleted: true },
        message: "ok",
      }));
    vi.stubGlobal("fetch", fetchMock);

    const binary = new File(["image"], "original.png", { type: "image/png" });
    const command = {
      allowInheritance: true,
      attributes: { alt: "Hero" },
      binaryFiles: [{ binaryName: "hero.png", file: binary }],
      definitionCode: "IMAGE",
      description: "Homepage hero image",
      expectedActiveResourceVersion: null,
      name: "Hero image",
      removedBinaryNames: [],
      resourceCode: "hero-image",
      scopeKind: "WORKSPACE" as const,
      workspaceCode: "TRON_CORE",
    };

    await expect(createResource(command)).resolves.toEqual(activation);
    const [, createRequest] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(createRequest.method).toBe("POST");
    expect(createRequest.body).toBeInstanceOf(FormData);
    const createForm = createRequest.body as FormData;
    expect(JSON.parse(String(createForm.get("command")))).toEqual({
      allowInheritance: true,
      attributes: { alt: "Hero" },
      definitionCode: "IMAGE",
      description: "Homepage hero image",
      expectedActiveResourceVersion: null,
      name: "Hero image",
      removedBinaryNames: [],
      resourceCode: "hero-image",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    });
    const uploaded = createForm.getAll("binary");
    expect(uploaded).toHaveLength(1);
    expect(uploaded[0]).toBeInstanceOf(File);
    expect((uploaded[0] as File).name).toBe("hero.png");
    expect(new Headers(createRequest.headers).has("Content-Type")).toBe(false);

    await expect(updateResource({
      ...command,
      binaryFiles: [],
      expectedActiveResourceVersion: "v1-B2c",
      removedBinaryNames: ["obsolete.png"],
    })).resolves.toMatchObject({
      activationResult: "COMMITTED",
      resourceVersion: "v2-C3d",
    });

    await expect(setResourceDefinitionEnabled({
      definitionCode: "IMAGE",
      enabled: false,
      expectedActiveDefinitionVersion: "v1-A1a",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    })).resolves.toMatchObject({ activationResult: "COMMITTED" });

    await expect(deleteResource({
      expectedActiveResourceVersion: "v2-C3d",
      expectedDraftResourceVersion: null,
      expectedDraftRevision: null,
      resourceCode: "hero-image",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    })).resolves.toEqual({ deleted: true, resourceCode: "hero-image" });

    await expect(deleteResourceDefinition({
      definitionCode: "CUSTOM_IMAGE",
      expectedActiveDefinitionVersion: "v1-A1a",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    })).resolves.toEqual({ definitionCode: "CUSTOM_IMAGE", deleted: true });

    const [, setEnabledRequest] = fetchMock.mock.calls[2] as [string, RequestInit];
    expect(setEnabledRequest.body).toBe(JSON.stringify({
      definitionCode: "IMAGE",
      enabled: false,
      expectedActiveDefinitionVersion: "v1-A1a",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    }));
    const [, deleteResourceRequest] = fetchMock.mock.calls[3] as [string, RequestInit];
    expect(deleteResourceRequest.body).toBe(JSON.stringify({
      expectedActiveResourceVersion: "v2-C3d",
      expectedDraftResourceVersion: null,
      expectedDraftRevision: null,
      resourceCode: "hero-image",
      scopeKind: "WORKSPACE",
      workspaceCode: "TRON_CORE",
    }));
  });
});
