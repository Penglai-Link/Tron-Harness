import { requestApi } from "@/shared/api/httpClient";
import type { SkillCandidate, SkillCandidateDetail } from "../model/SkillCandidate";

export function fetchSkillCandidates(signal?: AbortSignal) {
  return requestApi<SkillCandidate[]>("GET /api/v1/skills/candidates", { signal });
}

export function fetchSkillCandidate(candidateCode: string, signal?: AbortSignal) {
  return requestApi<SkillCandidateDetail>("GET /api/v1/skills/candidates/{candidateCode}", {
    pathParams: { candidateCode }, signal,
  });
}

export type SkillCandidateAction = "approve" | "reject" | "revise" | "retry-install";

export function actOnSkillCandidate(
  candidate: Pick<SkillCandidate, "candidateCode" | "revision" | "artifactHash">,
  action: SkillCandidateAction,
  explanation?: string,
) {
  return requestApi<SkillCandidateDetail>(`POST /api/v1/skills/candidates/{candidateCode}/${action}`, {
    pathParams: { candidateCode: candidate.candidateCode },
    body: {
      revision: candidate.revision,
      artifactHash: candidate.artifactHash,
      ...(action === "reject" ? { reason: explanation?.trim() ?? "" } : {}),
      ...(action === "revise" ? { feedback: explanation?.trim() ?? "" } : {}),
    },
  });
}
