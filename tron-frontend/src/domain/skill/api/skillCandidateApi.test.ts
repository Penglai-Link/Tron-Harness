// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { actOnSkillCandidate, fetchSkillCandidate, fetchSkillCandidates } from "./skillCandidateApi";

afterEach(() => { vi.unstubAllGlobals(); });
const identity = { candidateCode: "candidate /1", revision: 5, artifactHash: "reviewed-hash" };
const response = (data: unknown, status = 200) => new Response(JSON.stringify({ code: status === 200 ? "SUCCESS" : "CONFLICT", message: "Artifact changed", data }), { status, headers: { "Content-Type": "application/json" } });

describe("skill candidate API", () => {
  it("reads real list/detail endpoints and forwards query cancellation", async () => {
    const fetch = vi.fn().mockResolvedValueOnce(response([])).mockResolvedValueOnce(response(identity));
    vi.stubGlobal("fetch", fetch);
    const controller = new AbortController();
    await expect(fetchSkillCandidates(controller.signal)).resolves.toEqual([]);
    await expect(fetchSkillCandidate(identity.candidateCode, controller.signal)).resolves.toEqual(identity);
    expect(fetch.mock.calls[0][1]).toMatchObject({ method: "GET", signal: controller.signal });
    expect(new URL(fetch.mock.calls[1][0]).pathname).toBe("/api/v1/skills/candidates/candidate%20%2F1");
  });
  it.each(["approve", "reject", "revise", "retry-install"] as const)("binds %s to the exact reviewed revision and artifact", async (action) => {
    const fetch = vi.fn().mockResolvedValue(response(identity));
    vi.stubGlobal("fetch", fetch);
    await actOnSkillCandidate(identity, action, "  needs evidence  ");
    const [url, request] = fetch.mock.calls[0];
    expect(new URL(url).pathname).toBe(`/api/v1/skills/candidates/candidate%20%2F1/${action}`);
    expect(request.method).toBe("POST");
    expect(JSON.parse(request.body)).toEqual({ revision: 5, artifactHash: "reviewed-hash", ...(action === "reject" ? { reason: "needs evidence" } : {}), ...(action === "revise" ? { feedback: "needs evidence" } : {}) });
  });
  it("surfaces server conflicts without silently approving a newer artifact", async () => {
    const fetch = vi.fn().mockResolvedValue(response(null, 409));
    vi.stubGlobal("fetch", fetch);
    await expect(actOnSkillCandidate(identity, "approve")).rejects.toMatchObject({ status: 409, message: "Artifact changed" });
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});
