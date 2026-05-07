import type {
  PromptVariable,
  PromptRecord,
} from "@/domain/prompt/model/model";

export function getPromptVariables(): PromptVariable[] {
  return [
    { name: "{{customer_tier}}", type: "string", sample: "Enterprise" },
    { name: "{{incident_summary}}", type: "markdown", sample: "OAuth callback failures after deploy" },
  ];
}

const prompts: PromptRecord[] = [
  {
    id: "prompt-router",
    name: "LLM Router System",
    category: "System",
    inheritance: "Workspace override",
    updated: "Today 10:42",
    owner: "Platform",
    body: "You are routing a user request across approved workspace models. Prefer low-latency healthy providers and explain fallback decisions.",
    variables: { workspace: "enterprise-prod", primary_lang: "TypeScript", user_query: "Explain the hydration error in App.tsx" },
  },
  {
    id: "prompt-review",
    name: "Code Review Guidelines",
    category: "Tools",
    inheritance: "Inherited",
    updated: "Yesterday",
    owner: "Engineering",
    body: "Review changes for correctness, regressions, and missing tests. Lead with findings and cite file lines.",
    variables: { repo_name: "frontend-core", primary_lang: "TypeScript" },
  },
  {
    id: "prompt-persona",
    name: "Support Triage Persona",
    category: "Persona",
    inheritance: "Local",
    updated: "Apr 29",
    owner: "Operations",
    body: "Classify incidents, map owner teams, and propose the least disruptive remediation path.",
    variables: { severity: "P2", workspace: "enterprise-prod" },
  },
];

export function listPrompts() {
  return prompts;
}
