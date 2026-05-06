import type { ReactNode } from "react";

import { MainAgentPage } from "./agent";
import { AuditLogsPage } from "./admin/audit-logs";
import { RoleMatrixPage } from "./admin/roles";
import { UsersPage } from "./admin/users";
import { AppsPage } from "./app";
import { ImportApplicationGithubPage } from "./app/import-github";
import { ImportApplicationLocalPage } from "./app/import-local";
import { LoginPage } from "./auth/login";
import { AutomationsPage } from "./automation";
import { CreateAutomationPage } from "./automation/new";
import { ChatSessionExecutionPage } from "./chat/execution";
import { ChatSessionReasoningPage } from "./chat";
import { DashboardPage } from "./dashboard";
import { KnowledgeGraphPage } from "./knowledge-graph";
import { KnowledgeGraphDomainPage } from "./knowledge-graph/domain-filtering";
import { MemoryDetailsPage } from "./memory/details";
import { MemoryPage } from "./memory";
import { AddModelPage } from "./model/new";
import { ModelRegistryPage } from "./model";
import { ObservabilityPage } from "./observability";
import { CreatePromptPage } from "./prompt/new";
import { PromptsPage } from "./prompt";
import { ResourcesPage } from "./resource";
import { CreateSkillPage } from "./skill/new";
import { SkillsPage } from "./skill";
import { TaskDetailsPage } from "./task/detail";
import { TasksImportPage } from "./task/import";
import { CreateTaskPage } from "./task/new";
import { TasksKanbanPage } from "./task";
import { VariableDetailsPage } from "./variable/details";
import { VariablesPage } from "./variable";

type RoutePageComponent = () => ReactNode;

export const routePages: Record<string, RoutePageComponent> = {
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

export { LoginPage } from "./auth/login";
export { TasksImportPage } from "./task/import";
