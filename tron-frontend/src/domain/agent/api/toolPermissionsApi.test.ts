import { beforeEach, expect, it, vi } from "vitest";
import { requestApi } from "@/shared/api/httpClient";
import { fetchToolPermissions, updateToolPermissions } from "./toolPermissionsApi";
vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn() }));
beforeEach(() => vi.mocked(requestApi).mockReset());
it("reads PascalCase wire enums into UI values", async () => {
  vi.mocked(requestApi).mockResolvedValue({ permissions: [{ toolCode: "bash.exec", permission: "Ask", confirmLevel: "High" }] });
  expect(await fetchToolPermissions()).toEqual([{ toolCode: "bash.exec", permission: "ask", confirmLevel: "high" }]);
  expect(requestApi).toHaveBeenCalledWith("GET /api/v1/tools/permissions", { signal: undefined });
});
it("writes the full snapshot using PascalCase enum values", async () => {
  vi.mocked(requestApi).mockResolvedValue({ permissions: [{ toolCode: "bash.exec", permission: "No", confirmLevel: "Safe" }] });
  await updateToolPermissions({ permissions: [{ toolCode: "bash.exec", permission: "no", confirmLevel: "safe" }] });
  expect(requestApi).toHaveBeenCalledWith("PUT /api/v1/tools/permissions", { body: { permissions: [{ toolCode: "bash.exec", permission: "No", confirmLevel: "Safe" }] } });
});
