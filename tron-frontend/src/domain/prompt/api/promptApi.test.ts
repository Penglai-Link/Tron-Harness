import { beforeEach, describe, expect, it, vi } from "vitest";

import { requestApi } from "@/shared/api/httpClient";
import {
  createPrompt,
  fetchPromptPage,
  promptApiEndpoints,
  updatePrompt,
} from "./promptApi";

vi.mock("@/shared/api/httpClient", () => ({
  requestApi: vi.fn(() => Promise.resolve({})),
}));

describe("global prompt API", () => {
  beforeEach(() => {
    vi.mocked(requestApi).mockClear();
  });

  it("exposes only platform prompt management endpoints", () => {
    expect(promptApiEndpoints).toEqual({
      analyzeTemplate: "POST /api/v1/prompts/analyze-template",
      create: "POST /api/v1/admin/platform/prompts/create",
      delete: "POST /api/v1/admin/platform/prompts/delete",
      detail: "POST /api/v1/admin/platform/prompts/detail",
      duplicate: "POST /api/v1/admin/platform/prompts/duplicate",
      page: "POST /api/v1/admin/platform/prompts/page",
      setEnabled: "POST /api/v1/admin/platform/prompts/set-enabled",
      translateDraft: "POST /api/v1/prompts/translate-draft",
      update: "POST /api/v1/admin/platform/prompts/update",
    });
  });

  it("sends paging criteria without workspace scope and supports abort", async () => {
    const signal = new AbortController().signal;
    const payload = { current: 2, size: 20, category: "SYSTEM" as const };

    await fetchPromptPage(payload, signal);

    expect(requestApi).toHaveBeenCalledWith(promptApiEndpoints.page, {
      body: payload,
      signal,
    });
  });

  it("keeps create and update payloads within the global prompt model", async () => {
    const variable = {
      description: "Routing request",
      managedVariableKey: "PROMPT_TEST_JSON",
      name: "request",
      runtimeProvider: null,
      sourceKind: "MANAGED" as const,
      type: "JSON" as const,
    };
    await createPrompt({
      category: "SYSTEM",
      description: "",
      displayName: "Router",
      enabled: true,
      promptBody: "Route {{request}}",
      variables: [variable],
    });
    await updatePrompt({
      description: "Updated",
      displayName: "Router",
      enabled: true,
      promptBody: "Route {{request}}",
      promptCode: "PROMPT-1",
      expectedVersion: 3,
      variables: [variable],
    });

    expect(requestApi).toHaveBeenNthCalledWith(1, promptApiEndpoints.create, {
      body: expect.objectContaining({ variables: [variable] }),
    });
    expect(requestApi).toHaveBeenNthCalledWith(2, promptApiEndpoints.update, {
      body: expect.not.objectContaining({ category: expect.anything(), workspaceCode: expect.anything() }),
    });
  });
});
