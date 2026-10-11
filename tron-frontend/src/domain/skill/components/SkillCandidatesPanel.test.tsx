// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SkillCandidatesPanel } from "./SkillCandidatesPanel";
import type { SkillCandidateDetail } from "../model/SkillCandidate";

const api = vi.hoisted(() => ({ list: vi.fn(), detail: vi.fn(), action: vi.fn() }));
vi.mock("../api/skillCandidateApi", () => ({ fetchSkillCandidates: api.list, fetchSkillCandidate: api.detail, actOnSkillCandidate: api.action }));
vi.mock("../api/skillApi", () => ({ notifyCapabilityResourcesChanged: vi.fn() }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/shared", () => ({ SurfaceCard: ({ children }: { children: React.ReactNode }) => <div>{children}</div> }));
vi.mock("@/components/files/CodeEditor", () => ({ CodeEditor: ({ value }: { value: string }) => <pre>{value}</pre> }));
const fixture: SkillCandidateDetail = {
  candidateCode: "candidate-1", dudeCode: "memory-dude", batchCode: "batch-1", skillName: "repair-workflow",
  operation: "Update", revision: 3, artifactHash: "reviewed-hash", status: "PendingApproval", baseArtifactHash: "base-hash",
  creator: { skillName: "skill-creator", version: "1", contentHash: "creator-hash" },
  files: [{ path: "SKILL.md", content: "COMPLETE NEW CONTENT", encoding: "Utf8" }],
  installedFiles: [{ path: "SKILL.md", content: "COMPLETE OLD CONTENT", encoding: "Utf8" }], evidence: [],
  validation: { staticPassed: true, behaviorPassed: true, artifactHash: "reviewed-hash", checks: [] },
  reason: null, feedback: null, lastError: null, createdAt: 1, updatedAt: 2,
};
function mount(path = "/settings/skills?tab=approvals&candidateCode=candidate-1") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const view = render(<QueryClientProvider client={client}><MemoryRouter initialEntries={[path]}><SkillCandidatesPanel /></MemoryRouter></QueryClientProvider>);
  return { ...view, client };
}
beforeEach(() => { vi.clearAllMocks(); api.list.mockResolvedValue([fixture]); api.detail.mockResolvedValue(fixture); api.action.mockResolvedValue({ ...fixture, status: "Installed" }); });
afterEach(() => cleanup());

describe("skill approval review", () => {
  it("shows full installed and proposed content and submits the reviewed artifact", async () => {
    mount();
    await screen.findByText("COMPLETE NEW CONTENT");
    expect(screen.getByText("COMPLETE OLD CONTENT")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "skillCandidates.approveInstall" }));
    await waitFor(() => expect(api.action).toHaveBeenCalledWith(fixture, "approve", ""));
  });
  it("disables approval for validation of a previous artifact", async () => {
    api.detail.mockResolvedValue({ ...fixture, validation: { ...fixture.validation!, artifactHash: "older-hash" } });
    mount();
    const approve = await screen.findByRole("button", { name: "skillCandidates.approveInstall" });
    expect((approve as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(approve);
    expect(api.action).not.toHaveBeenCalled();
  });
  it("requires feedback for revision and keeps errors visible", async () => {
    api.action.mockRejectedValue(new Error("Artifact changed"));
    mount();
    const revise = await screen.findByRole("button", { name: "skillCandidates.requestRevision" });
    expect((revise as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("skillCandidates.feedbackLabel"), { target: { value: "Test the failure path" } });
    fireEvent.click(revise);
    await screen.findByText("Artifact changed");
    expect(api.action).toHaveBeenCalledWith(fixture, "revise", "Test the failure path");
    expect((screen.getByLabelText("skillCandidates.feedbackLabel") as HTMLTextAreaElement).value).toBe("Test the failure path");
  });
  it("retries a failed detail request without showing fabricated candidate content", async () => {
    api.detail.mockRejectedValueOnce(new Error("Unavailable"));
    mount();
    await screen.findByText("Unavailable");
    expect(screen.queryByText("COMPLETE NEW CONTENT")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "skillCandidates.retry" }));
    await screen.findByText("COMPLETE NEW CONTENT");
    expect(api.detail).toHaveBeenCalledTimes(2);
  });
  it("offers installation retry after approval without offering approval again", async () => {
    api.detail.mockResolvedValue({ ...fixture, status: "InstallFailed", lastError: "Disk unavailable" });
    mount();
    await screen.findByText("Disk unavailable");
    expect(screen.queryByRole("button", { name: "skillCandidates.approveInstall" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "skillCandidates.retryInstall" }));
    await waitFor(() => expect(api.action).toHaveBeenCalledWith(expect.objectContaining({ artifactHash: "reviewed-hash" }), "retry-install", ""));
  });
});
