import {
  getSkillSchemaPreview,
  listCapabilities,
} from "@/domain/skill/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
export type {
  StatusTone
} from "@/domain/skill/constants/StatusTone";
export type {
  SkillSchemaPreview,
  CapabilityRecord
} from "@/domain/skill/model/SkillModel";

export const skillApiEndpoints = {
  listCapabilities: "GET /api/v1/capabilities",
  getSkillSchemaPreview: "GET /api/v1/skills/schema-preview",
} as const;

export function useSkillSchemaPreview() {
  return useServiceQuery({
    domain: "skill",
    endpoint: skillApiEndpoints.getSkillSchemaPreview,
    mockData: () => getSkillSchemaPreview(),
    deps: [],
  });
}

export function useCapabilities() {
  return useServiceQuery({
    domain: "skill",
    endpoint: skillApiEndpoints.listCapabilities,
    mockData: () => listCapabilities(),
    deps: [],
  });
}
