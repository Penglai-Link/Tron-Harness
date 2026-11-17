export type SkillCandidateStatus =
  | "WaitingDependency" | "Authoring" | "ValidationFailed" | "PendingApproval"
  | "Approved" | "Installing" | "InstallFailed" | "Installed" | "Rejected" | "RevisionRequested";

export interface SkillCandidateFile {
  path: string;
  content: string;
  encoding: "Utf8" | "Base64";
}

export interface SkillCandidate {
  candidateCode: string;
  assistantCode: string;
  batchCode: string;
  skillName: string;
  operation: "Create" | "Update";
  revision: number;
  artifactHash: string;
  status: SkillCandidateStatus;
  baseArtifactHash: string | null;
  creator: { skillName: string; version: string; contentHash: string } | null;
  files: SkillCandidateFile[];
  evidence: { sourceCode: string; workspaceCode: string; version: string; excerpt: string }[];
  validation: {
    staticPassed: boolean;
    behaviorPassed: boolean;
    checks: { name: string; passed: boolean; evidence: string }[];
    artifactHash: string;
  } | null;
  reason: string | null;
  feedback: string | null;
  lastError: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface SkillCandidateDetail extends SkillCandidate {
  installedFiles: SkillCandidateFile[];
}

/** Approval is tied to the validated artifact being reviewed. The server rechecks it. */
export function canApproveSkillCandidate(candidate: SkillCandidate): boolean {
  return candidate.status === "PendingApproval"
    && candidate.validation?.staticPassed === true
    && candidate.validation.behaviorPassed
    && candidate.validation.artifactHash === candidate.artifactHash;
}

export function compareSkillCandidateFiles(candidate: SkillCandidateDetail) {
  const installed = new Map(candidate.installedFiles.map((file) => [file.path, file]));
  const proposed = new Map(candidate.files.map((file) => [file.path, file]));
  return [...new Set([...proposed.keys(), ...installed.keys()])].sort().map((path) => {
    const before = installed.get(path);
    const after = proposed.get(path);
    const change = !before ? "Added" : !after ? "Removed"
      : before.content === after.content && before.encoding === after.encoding ? "Unchanged" : "Changed";
    return { path, before, after, change };
  });
}
