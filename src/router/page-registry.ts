import type { ReactNode } from "react";

import { MainAgentPage } from "@/domain/agent/page";
import { AuditLogsPage } from "@/domain/admin/page/audit-logs";
import { RoleMatrixPage } from "@/domain/admin/page/roles";
import { UsersPage } from "@/domain/admin/page/users";
import { AppsPage } from "@/domain/app/page";
import { ImportApplicationGithubPage } from "@/domain/app/page/import-github";
import { ImportApplicationLocalPage } from "@/domain/app/page/import-local";
import { LoginPage } from "@/domain/auth/page/login";
import { AutomationsPage } from "@/domain/automation/page";
import { CreateAutomationPage } from "@/domain/automation/page/new";
import { ChatSessionExecutionPage } from "@/domain/chat/page/execution";
import { ChatSessionReasoningPage } from "@/domain/chat/page";
import { DashboardPage } from "@/domain/dashboard/page";
import { KnowledgeGraphPage } from "@/domain/knowledge-graph/page";
import { KnowledgeGraphDomainPage } from "@/domain/knowledge-graph/page/domain-filtering";
import { MemoryDetailsPage } from "@/domain/memory/page/details";
import { MemoryPage } from "@/domain/memory/page";
import { AddModelPage } from "@/domain/model/page/new";
import { ModelRegistryPage } from "@/domain/model/page";
import { ObservabilityPage } from "@/domain/observability/page";
import { CreatePromptPage } from "@/domain/prompt/page/new";
import { PromptsPage } from "@/domain/prompt/page";
import { ResourcesPage } from "@/domain/resource/page";
import { CreateSkillPage } from "@/domain/skill/page/new";
import { SkillsPage } from "@/domain/skill/page";
import { TaskDetailsPage } from "@/domain/task/page/detail";
import { TasksImportPage } from "@/domain/task/page/import";
import { CreateTaskPage } from "@/domain/task/page/new";
import { TasksKanbanPage } from "@/domain/task/page";
import { VariableDetailsPage } from "@/domain/variable/page/details";
import { VariablesPage } from "@/domain/variable/page";

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

export { LoginPage } from "@/domain/auth/page/login";
export { TasksImportPage } from "@/domain/task/page/import";
