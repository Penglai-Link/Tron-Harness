import type { VariableRecord } from "@/domain/variable/model/model";

const variables: VariableRecord[] = [
  { id: "openai", key: "OPENAI_API_KEY", scope: "Workspace", type: "Secret", valuePreview: "sk-...a92f", status: "Active", updated: "Today", usage: ["LLM Router System", "Code Review Guidelines"] },
  { id: "region", key: "DEFAULT_REGION", scope: "Inherited", type: "String", valuePreview: "us-east-1", status: "Active", updated: "Yesterday", usage: ["Failover Controller", "Trace Exporter"] },
  { id: "timeout", key: "ROUTER_TIMEOUT_MS", scope: "Local", type: "Number", valuePreview: "400", status: "Override", updated: "Apr 30", usage: ["LLM Router System"] },
];

export function listVariables() {
  return variables;
}
