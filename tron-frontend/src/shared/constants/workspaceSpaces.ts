import type { PrimarySpace, SecondarySpace } from "@/shared/types/WorkspaceSpace";

export const defaultPrimarySpaceId = "ps-enterprise";
export const defaultSecondarySpaceId = "ss-tron-main";

export const workspaceSpaceApiContracts = {
  listPrimarySpaces: "GET /api/v1/primary-spaces",
  listSecondarySpaces: "GET /api/v1/primary-spaces/{primarySpaceId}/secondary-spaces",
} as const;

export const primarySpaceMocks: PrimarySpace[] = [
  {
    id: "ps-enterprise",
    name: "Enterprise Workspace",
    description: "Global governance, security, and shared AI resource policy.",
    region: "Global",
    status: "Active",
  },
  {
    id: "ps-research",
    name: "Research Workspace",
    description: "R&D model evaluation, prompt experiments, and internal workspaces.",
    region: "APAC",
    status: "Active",
  },
  {
    id: "ps-customer-ops",
    name: "Customer Ops Workspace",
    description: "Customer-facing agent operations and support workflows.",
    region: "US",
    status: "Active",
  },
  {
    id: "ps-compliance",
    name: "Compliance Workspace",
    description: "Audit, retention, permission, and regulated resource controls.",
    region: "EU",
    status: "Review",
  },
];

export const secondarySpaceMocks: SecondarySpace[] = [
  { id: "ss-tron-main", primarySpaceId: "ps-enterprise", name: "Tron", description: "Main Console", inheritedResources: 24, localOverrides: 5, status: "Active" },
  { id: "ss-agent-platform", primarySpaceId: "ps-enterprise", name: "Agent Platform", description: "Runtime Governance", inheritedResources: 18, localOverrides: 3, status: "Active" },
  { id: "ss-security-lab", primarySpaceId: "ps-enterprise", name: "Security Lab", description: "Policy Sandbox", inheritedResources: 16, localOverrides: 7, status: "Review" },
  { id: "ss-model-research", primarySpaceId: "ps-research", name: "Model Research", description: "Evaluation Console", inheritedResources: 12, localOverrides: 8, status: "Active" },
  { id: "ss-prompt-lab", primarySpaceId: "ps-research", name: "Prompt Lab", description: "Experiment Studio", inheritedResources: 10, localOverrides: 11, status: "Active" },
  { id: "ss-support-ai", primarySpaceId: "ps-customer-ops", name: "Support AI", description: "Customer Agents", inheritedResources: 21, localOverrides: 4, status: "Active" },
  { id: "ss-success-ops", primarySpaceId: "ps-customer-ops", name: "Success Ops", description: "Renewal Workflows", inheritedResources: 15, localOverrides: 6, status: "Active" },
  { id: "ss-audit-center", primarySpaceId: "ps-compliance", name: "Audit Center", description: "Evidence Review", inheritedResources: 19, localOverrides: 2, status: "Review" },
];

export function getSecondarySpaceMocks(primarySpaceId: PrimarySpace["id"]) {
  return secondarySpaceMocks.filter((space) => space.primarySpaceId === primarySpaceId);
}
