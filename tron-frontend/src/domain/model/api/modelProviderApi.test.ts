import { beforeEach, expect, it, vi } from "vitest";

const request = vi.hoisted(() => vi.fn().mockResolvedValue({}));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: request }));
import { createModelProviderAuthorization, fetchModelProviderAuthorization, pollModelProviderAuthorization } from "./modelProviderApi";
import { createPersonAiAuthorization, pollPersonAiAuthorization } from "@/domain/profile/api/profileApi";

beforeEach(() => request.mockClear());

it("keeps provider reauthorization on the profile-preserving endpoints", async () => {
  await createModelProviderAuthorization("PersonAi");
  await fetchModelProviderAuthorization("PersonAi", "session");
  await pollModelProviderAuthorization("PersonAi", "session");
  expect(request.mock.calls.map(call => call[0])).toEqual([
    "POST /api/v1/model-providers/{providerCode}/reauthorization-sessions",
    "GET /api/v1/model-providers/{providerCode}/reauthorization-sessions/{sessionId}",
    "POST /api/v1/model-providers/{providerCode}/reauthorization-sessions/{sessionId}/poll",
  ]);
});

it("keeps adding a profile on the account registration endpoints", async () => {
  await createPersonAiAuthorization();
  await pollPersonAiAuthorization("session");
  expect(request.mock.calls.map(call => call[0])).toEqual([
    "POST /api/v1/model-providers/{providerCode}/authorization-sessions",
    "POST /api/v1/model-providers/{providerCode}/authorization-sessions/{sessionId}/poll",
  ]);
});
