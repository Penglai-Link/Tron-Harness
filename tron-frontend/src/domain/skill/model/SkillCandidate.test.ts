import { describe, expect, it } from "vitest";
import { canApproveSkillCandidate, compareSkillCandidateFiles, type SkillCandidateDetail } from "./SkillCandidate";

export const candidate: SkillCandidateDetail = {
  candidateCode: "candidate-1", dudeCode: "memory-dude", batchCode: "batch-1", skillName: "testing-skill",
  operation: "Update", revision: 2, artifactHash: "current-hash", status: "PendingApproval", baseArtifactHash: "base-hash",
  creator: { skillName: "skill-creator", version: "1", contentHash: "creator-hash" },
  files: [{ path: "SKILL.md", content: "new content", encoding: "Utf8" }, { path: "scripts/new.py", content: "print(1)", encoding: "Utf8" }],
  installedFiles: [{ path: "SKILL.md", content: "old content", encoding: "Utf8" }, { path: "scripts/old.py", content: "print(0)", encoding: "Utf8" }],
  evidence: [], validation: { staticPassed: true, behaviorPassed: true, artifactHash: "current-hash", checks: [] },
  reason: null, feedback: null, lastError: null, createdAt: 1, updatedAt: 2,
};

describe("skill artifact review", () => {
  it("requires validation of the exact pending revision before approving", () => {
    expect(canApproveSkillCandidate(candidate)).toBe(true);
    expect(canApproveSkillCandidate({ ...candidate, validation: null })).toBe(false);
    expect(canApproveSkillCandidate({ ...candidate, status: "Approved" })).toBe(false);
    expect(canApproveSkillCandidate({ ...candidate, validation: { ...candidate.validation!, artifactHash: "old-hash" } })).toBe(false);
    expect(canApproveSkillCandidate({ ...candidate, validation: { ...candidate.validation!, behaviorPassed: false } })).toBe(false);
  });
  it("includes removed files and keeps exact content when comparing an update", () => {
    expect(compareSkillCandidateFiles(candidate)).toMatchObject([
      { path: "SKILL.md", change: "Changed", before: { content: "old content" }, after: { content: "new content" } },
      { path: "scripts/new.py", change: "Added", before: undefined },
      { path: "scripts/old.py", change: "Removed", after: undefined },
    ]);
  });
  it("treats encoding changes as an artifact change", () => {
    const file = { path: "asset", content: "abcd", encoding: "Utf8" as const };
    expect(compareSkillCandidateFiles({ ...candidate, files: [file], installedFiles: [{ ...file, encoding: "Base64" }] })[0].change).toBe("Changed");
  });
});
