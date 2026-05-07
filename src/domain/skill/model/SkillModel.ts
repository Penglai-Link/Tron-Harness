export interface SkillSchemaPreview {
  name: string;
  fields: { name: string; type: string; required: boolean }[];
}

export interface CapabilityRecord {
  id: string;
  mode: "skills" | "mcp";
  name: string;
  description: string;
  provider: string;
  status: string;
  enabled: boolean;
  latency: string;
  schema: string;
  activity: string[];
}
