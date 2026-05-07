import type { ReactNode } from "react";

import { MainAgentPage } from "@/domain/agent/pages/MainAgentPage";
import { AuditLogsPage } from "@/domain/admin/pages/AuditLogsPage";
import { RoleMatrixPage } from "@/domain/admin/pages/RoleMatrixPage";
import { UsersPage } from "@/domain/admin/pages/UsersPage";
import { AppsPage } from "@/domain/app/pages/AppsPage";
import { ImportApplicationGithubPage } from "@/domain/app/pages/ImportApplicationGithubPage";
import { ImportApplicationLocalPage } from "@/domain/app/pages/ImportApplicationLocalPage";
import { LoginPage } from "@/domain/auth/pages/LoginPage";
import { AutomationsPage } from "@/domain/automation/pages/AutomationsPage";
import { CreateAutomationPage } from "@/domain/automation/pages/CreateAutomationPage";
import { ChatSessionExecutionPage } from "@/domain/chat/pages/ChatSessionExecutionPage";
import { ChatSessionReasoningPage } from "@/domain/chat/pages/ChatSessionReasoningPage";
import { DashboardPage } from "@/domain/dashboard/pages/DashboardPage";
import { KnowledgeGraphPage } from "@/domain/knowledge-graph/pages/KnowledgeGraphPage";
import { KnowledgeGraphDomainPage } from "@/domain/knowledge-graph/pages/KnowledgeGraphDomainPage";
import { MemoryDetailsPage } from "@/domain/memory/pages/MemoryDetailsPage";
import { MemoryPage } from "@/domain/memory/pages/MemoryPage";
import { AddModelPage } from "@/domain/model/pages/AddModelPage";
import { ModelRegistryPage } from "@/domain/model/pages/ModelRegistryPage";
import { ObservabilityPage } from "@/domain/observability/pages/ObservabilityPage";
import { CreatePromptPage } from "@/domain/prompt/pages/CreatePromptPage";
import { PromptsPage } from "@/domain/prompt/pages/PromptsPage";
import { ResourcesPage } from "@/domain/resource/pages/ResourcesPage";
import { CreateSkillPage } from "@/domain/skill/pages/CreateSkillPage";
import { SkillsPage } from "@/domain/skill/pages/SkillsPage";
import { TaskDetailsPage } from "@/domain/task/pages/TaskDetailsPage";
import { TasksImportPage } from "@/domain/task/pages/TasksImportPage";
import { CreateTaskPage } from "@/domain/task/pages/CreateTaskPage";
import { TasksKanbanPage } from "@/domain/task/pages/TasksKanbanPage";
import { VariableDetailsPage } from "@/domain/variable/pages/VariableDetailsPage";
import { VariablesPage } from "@/domain/variable/pages/VariablesPage";

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

export { LoginPage } from "@/domain/auth/pages/LoginPage";
export { TasksImportPage } from "@/domain/task/pages/TasksImportPage";
