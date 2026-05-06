import type { ReactNode } from "react";

import { AuditLogsPage } from "./admin-audit-logs";
import { RoleMatrixPage } from "./admin-roles";
import { UsersPage } from "./admin-users";
import { AppsPage } from "./apps";
import { ImportApplicationGithubPage } from "./apps-import-github";
import { ImportApplicationLocalPage } from "./apps-import-local";
import { AutomationsPage } from "./automations";
import { CreateAutomationPage } from "./automations-new";
import { ChatSessionReasoningPage } from "./chats";
import { ChatSessionExecutionPage } from "./chats-execution";
import { DashboardPage } from "./dashboard";
import { KnowledgeGraphPage } from "./knowledge-graph";
import { KnowledgeGraphDomainPage } from "./knowledge-graph-domain-filtering";
import { LoginPage } from "./login";
import { MainAgentPage } from "./main-agent";
import { MemoryPage } from "./memory";
import { MemoryDetailsPage } from "./memory-details";
import { ModelRegistryPage } from "./models";
import { AddModelPage } from "./models-new";
import { ObservabilityPage } from "./observability";
import { PromptsPage } from "./prompts";
import { CreatePromptPage } from "./prompts-new";
import { ResourcesPage } from "./resources";
import { SkillsPage } from "./skills";
import { CreateSkillPage } from "./skills-new";
import { TasksKanbanPage } from "./tasks";
import { TaskDetailsPage } from "./tasks-detail";
import { TasksImportPage } from "./tasks-import";
import { CreateTaskPage } from "./tasks-new";
import { VariablesPage } from "./variables";
import { VariableDetailsPage } from "./variables-details";

type ConsolePageComponent = () => ReactNode;

export const consolePages: Record<string, ConsolePageComponent> = {
  "admin.auditLogs": AuditLogsPage,
  "admin.roles": RoleMatrixPage,
  "admin.users": UsersPage,
  "apps.index": AppsPage,
  "apps.import.github": ImportApplicationGithubPage,
  "apps.import.local": ImportApplicationLocalPage,
  "automations.index": AutomationsPage,
  "automations.new": CreateAutomationPage,
  "chats.reasoning": ChatSessionReasoningPage,
  "chats.execution": ChatSessionExecutionPage,
  "dashboard.index": DashboardPage,
  "knowledgeGraph.index": KnowledgeGraphPage,
  "knowledgeGraph.domainFiltering": KnowledgeGraphDomainPage,
  "auth.login": LoginPage,
  "agent.main": MainAgentPage,
  "memory.index": MemoryPage,
  "memory.details": MemoryDetailsPage,
  "models.index": ModelRegistryPage,
  "models.new": AddModelPage,
  "observability.index": ObservabilityPage,
  "prompts.index": PromptsPage,
  "prompts.new": CreatePromptPage,
  "resources.index": ResourcesPage,
  "skills.index": SkillsPage,
  "skills.new": CreateSkillPage,
  "tasks.index": TasksKanbanPage,
  "tasks.detail": TaskDetailsPage,
  "tasks.new": CreateTaskPage,
  "variables.index": VariablesPage,
  "variables.details": VariableDetailsPage,
};

export { LoginPage } from "./login";
export { TasksImportPage } from "./tasks-import";
