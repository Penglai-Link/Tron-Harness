// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
const request = vi.hoisted(() => vi.fn(async () => ({})));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: request }));
import { createKnowledgeLibrary, deleteKnowledgeLibrary, indexKnowledgeDocument, listKnowledgeLibraries, previewKnowledgeDocument, setKnowledgeLibraryGrants, updateKnowledgeDocument, uploadKnowledgeDocument } from "./knowledgeBaseApi";

beforeEach(() => request.mockClear());
describe("knowledge base product API", () => {
  it("scopes all reads and writes to the selected workspace", async () => {
    await listKnowledgeLibraries("space-a");
    expect(request).toHaveBeenLastCalledWith("GET /api/v1/knowledge-bases", expect.objectContaining({ queryParams: { workspaceCode: "space-a" } }));
    await createKnowledgeLibrary("space-a", "Policies", "Internal");
    expect(request).toHaveBeenLastCalledWith("POST /api/v1/knowledge-bases", { body: { workspaceCode: "space-a", name: "Policies", description: "Internal" } });
    await setKnowledgeLibraryGrants("space-a", "kb-a", []);
    expect(request).toHaveBeenLastCalledWith("PUT /api/v1/knowledge-bases/{id}/grants", { pathParams: { id: "kb-a" }, body: { workspaceCode: "space-a", agentCodes: [] } });
    await deleteKnowledgeLibrary("space-a", "kb-a");
    expect(request).toHaveBeenLastCalledWith("DELETE /api/v1/knowledge-bases/{id}", { pathParams: { id: "kb-a" }, queryParams: { workspaceCode: "space-a" } });
  });
  it("uploads the original file with multipart without a parser or model endpoint", async () => {
    const file = new File(["# Policy"], "policy.md");
    await uploadKnowledgeDocument("space-a", "kb-a", file);
    const [endpoint, options] = request.mock.calls.at(-1)! as unknown as [string, { body: FormData; pathParams: object }];
    expect(endpoint).toBe("POST /api/v1/knowledge-bases/{id}/documents");
    expect(options.body.get("workspaceCode")).toBe("space-a");
    expect((options.body.get("file") as File).name).toBe("policy.md");
  });
  it("distinguishes inherited config, preview, full rebuild and failed-stage retry", async () => {
    await updateKnowledgeDocument("space-a", "kb-a", "doc-a", null);
    expect(request).toHaveBeenLastCalledWith("PATCH /api/v1/knowledge-bases/{id}/documents/{documentId}", expect.objectContaining({ body: { workspaceCode: "space-a", chunkConfig: null } }));
    await updateKnowledgeDocument("space-a", "kb-a", "doc-a", null, null, "Ocr");
    expect(request).toHaveBeenLastCalledWith("PATCH /api/v1/knowledge-bases/{id}/documents/{documentId}", expect.objectContaining({ body: { workspaceCode: "space-a", chunkConfig: null, parsingTier: null, ocrMode: "Ocr" } }));
    await updateKnowledgeDocument("space-a", "kb-a", "doc-a", null, null, null);
    expect(request).toHaveBeenLastCalledWith("PATCH /api/v1/knowledge-bases/{id}/documents/{documentId}", expect.objectContaining({ body: { workspaceCode: "space-a", chunkConfig: null, parsingTier: null, ocrMode: null } }));
    await previewKnowledgeDocument("space-a", "kb-a", "doc-a");
    expect(request).toHaveBeenLastCalledWith("POST /api/v1/knowledge-bases/{id}/documents/{documentId}/preview", expect.any(Object));
    await indexKnowledgeDocument("space-a", "kb-a", "doc-a");
    expect(request).toHaveBeenLastCalledWith("POST /api/v1/knowledge-bases/{id}/documents/{documentId}/index", expect.any(Object));
    await indexKnowledgeDocument("space-a", "kb-a", "doc-a", true);
    expect(request).toHaveBeenLastCalledWith("POST /api/v1/knowledge-bases/{id}/documents/{documentId}/retry", expect.any(Object));
  });
});
