import { getSkillSchemaPreview } from "@/domain/skill/mock/data";
import { useServiceQuery } from "@/shared/api/serviceConfig";
import { skillApiEndpoints } from "@/domain/skill/api";

export function useSkillSchemaPreview() {
  return useServiceQuery({
    domain: "skill",
    endpoint: skillApiEndpoints.getSkillSchemaPreview,
    mockData: () => getSkillSchemaPreview(),
    deps: [],
  });
}
