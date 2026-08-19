// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

import { adaptResourceRecord } from "@/domain/resource/adapters";
import {
  createResource,
  deleteResource,
  downloadResourceBinary,
  pageResources,
  updateResource,
  uploadResourceBinary,
} from "@/domain/resource/api";
import { setApiAccessToken } from "@/shared/api/httpClient";

const pageRequest = {
  current: 1,
  kinds: [],
  search: null,
  size: 20,
  sourceKind: null,
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
  it("adapts the wire record into the domain model", () => {
    const resource = adaptResourceRecord({
      allowInheritance: true,
      attributes: { pageCount: 12 },
      createdAt: 1_767_225_600_000,
      description: "Quarterly report",
      file: { byteSize: 184_320, fileExtension: "pdf", fileName: "report.pdf" },
      kind: "Pdf",
      name: "Q3 report",
      resourceCode: "q3-report",
      sourceKind: "Local",
      updatedAt: 1_767_312_000_000,
      workspaceCode: "TRON_CORE",
    });

    expect(resource).toMatchObject({
      attributes: { pageCount: 12 },
      createdAt: new Date(1_767_225_600_000).toISOString(),
      file: { byteSize: 184_320, fileExtension: "pdf", fileName: "report.pdf" },
      kind: "PDF",
      updatedAt: new Date(1_767_312_000_000).toISOString(),
      workspaceCode: "TRON_CORE",
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

    await expect(pageResources(pageRequest)).resolves.toEqual({
      current: 1,
      records: [],
      size: 20,
      total: 0,
    });

    const [, request] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(request.method).toBe("POST");
    expect(request.body).toBe(JSON.stringify(pageRequest));
    const headers = new Headers(request.headers);
    expect(headers.get("Authorization")).toBe("Bearer resource-test-token");
    expect(headers.get("Content-Type")).toBe("application/json");

    await expect(pageResources(pageRequest)).rejects.toMatchObject({
      code: "-1",
      message: "Resource state conflicts with this operation",
      status: 409,
    });
  });

  it("uses FormData for create and binary upload and returns raw downloads as blobs", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { kind: "File", resourceCode: "readme" },
        message: "ok",
      }))
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: {
          action: "UPLOADED",
          byteSize: 7,
          fileName: "report.pdf",
          resourceCode: "readme",
          targetPath: null,
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

    const binary = new File(["payload"], "report.pdf", { type: "application/pdf" });
    await expect(createResource({
      allowInheritance: true,
      description: "Readme",
      file: binary,
      kind: "FILE",
      name: "Readme",
      resourceCode: "readme",
      workspaceCode: "TRON_CORE",
    })).resolves.toEqual({ kind: "File", resourceCode: "readme" });

    const [, createRequest] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(createRequest.body).toBeInstanceOf(FormData);
    const createForm = createRequest.body as FormData;
    expect(JSON.parse(String(createForm.get("command")))).toEqual({
      allowInheritance: true,
      description: "Readme",
      kind: "File",
      name: "Readme",
      resourceCode: "readme",
      workspaceCode: "TRON_CORE",
    });
    const uploaded = createForm.get("binary");
    expect(uploaded).toBeInstanceOf(File);
    expect((uploaded as File).name).toBe("report.pdf");

    await expect(uploadResourceBinary({
      file: binary,
      resourceCode: "readme",
      workspaceCode: "TRON_CORE",
    })).resolves.toMatchObject({
      action: "UPLOADED",
      byteSize: 7,
      fileName: "report.pdf",
    });

    const [, uploadRequest] = fetchMock.mock.calls[1] as [string, RequestInit];
    const uploadForm = uploadRequest.body as FormData;
    expect(uploadForm.get("workspaceCode")).toBe("TRON_CORE");
    expect(uploadForm.get("resourceCode")).toBe("readme");
    expect(uploadForm.get("file")).toBeInstanceOf(File);
    expect(new Headers(uploadRequest.headers).has("Content-Type")).toBe(false);

    const download = await downloadResourceBinary({
      resourceCode: "readme",
      workspaceCode: "TRON_CORE",
    });
    expect(download.blob.size).toBe(7);
    expect(download.contentDisposition).toContain("filename*=");
    expect(download.contentType).toBe("application/octet-stream");
  });

  it("keeps update and delete commands as JSON", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { kind: "Image", resourceCode: "hero-image" },
        message: "ok",
      }))
      .mockResolvedValueOnce(jsonResponse({
        code: "0",
        data: { deleted: true, resourceCode: "hero-image" },
        message: "ok",
      }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(updateResource({
      allowInheritance: false,
      description: "Homepage hero image",
      name: "Hero image",
      resourceCode: "hero-image",
      workspaceCode: "TRON_CORE",
    })).resolves.toEqual({ kind: "Image", resourceCode: "hero-image" });

    await expect(deleteResource({
      resourceCode: "hero-image",
      workspaceCode: "TRON_CORE",
    })).resolves.toEqual({ deleted: true, resourceCode: "hero-image" });

    const [, updateRequest] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(updateRequest.body).toBe(JSON.stringify({
      allowInheritance: false,
      description: "Homepage hero image",
      name: "Hero image",
      resourceCode: "hero-image",
      workspaceCode: "TRON_CORE",
    }));
    const [, deleteRequest] = fetchMock.mock.calls[1] as [string, RequestInit];
    expect(deleteRequest.body).toBe(JSON.stringify({
      resourceCode: "hero-image",
      workspaceCode: "TRON_CORE",
    }));
  });
});
