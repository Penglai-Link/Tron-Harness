import { beforeEach, describe, expect, it, vi } from "vitest";

import { requestApi } from "@/shared/api/httpClient";
import {
  createWorkspacePrompt,
  fetchWorkspacePromptPage,
  promptApiEndpoints,
  updateWorkspacePrompt,
} from "./promptApi";

vi.mock("@/shared/api/httpClient", () => ({
  requestApi: vi.fn(() => Promise.resolve({})),
}));

describe("prompt API", () => {
  beforeEach(() => {
    vi.mocked(requestApi).mockClear();
  });

  it("registers the frozen POST-only endpoint contract", () => {
    expect(promptApiEndpoints).toEqual({
      analyzeTemplate: "POST /api/v1/prompts/analyze-template",
      createPlatform: "POST /api/v1/admin/platform/prompts/create",
      createWorkspace: "POST /api/v1/prompts/workspace-create",
      deletePlatform: "POST /api/v1/admin/platform/prompts/delete",
      deleteWorkspace: "POST /api/v1/prompts/workspace-delete",
      detailPlatform: "POST /api/v1/admin/platform/prompts/detail",
      detailWorkspace: "POST /api/v1/prompts/workspace-detail",
      duplicatePlatform: "POST /api/v1/admin/platform/prompts/duplicate",
      duplicateWorkspace: "POST /api/v1/prompts/workspace-duplicate",
      pagePlatform: "POST /api/v1/admin/platform/prompts/page",
      pageWorkspace: "POST /api/v1/prompts/workspace-page",
      revertWorkspace: "POST /api/v1/prompts/workspace-revert",
      setAllowInheritanceWorkspace: "POST /api/v1/prompts/workspace-set-allow-inheritance",
      setEnabledPlatform: "POST /api/v1/admin/platform/prompts/set-enabled",
      setEnabledWorkspace: "POST /api/v1/prompts/workspace-set-enabled",
      updatePlatform: "POST /api/v1/admin/platform/prompts/update",
      updateWorkspace: "POST /api/v1/prompts/workspace-update",
      workspaceExport: "POST /api/v1/prompts/workspace-export",
    });
  });

  it("sends workspace paging criteria in a POST body and supports abort", async () => {
    const signal = new AbortController().signal;
    const payload = { current: 2, size: 20, workspaceCode: "WORKSPACE-1" };

    await fetchWorkspacePromptPage(payload, signal);

    expect(requestApi).toHaveBeenCalledWith(promptApiEndpoints.pageWorkspace, {
      body: payload,
      signal,
    });
  });

  it("keeps create and update payload boundaries exact", async () => {
    await createWorkspacePrompt({
      allowInheritance: true,
      category: "SYSTEM",
      description: "",
      displayName: "Router",
      enabled: true,
      promptBody: "Route {{request}}",
      variables: [{ description: "Routing request", name: "request", type: "JSON" }],
      workspaceCode: "WORKSPACE-1",
    });
    await updateWorkspacePrompt({
      description: "Updated",
      displayName: "Router",
      enabled: true,
      promptBody: "Route {{request}}",
      promptCode: "PROMPT-1",
      expectedVersion: 3,
      variables: [{ description: "Routing request", name: "request", type: "JSON" }],
      workspaceCode: "WORKSPACE-1",
    });

    expect(requestApi).toHaveBeenNthCalledWith(1, promptApiEndpoints.createWorkspace, {
      body: expect.objectContaining({
        variables: [{ description: "Routing request", name: "request", type: "JSON" }],
      }),
    });
    expect(requestApi).toHaveBeenNthCalledWith(2, promptApiEndpoints.updateWorkspace, {
      body: expect.not.objectContaining({ category: expect.anything() }),
    });
  });
});
