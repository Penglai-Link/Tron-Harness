import type { MemoryWorkDomainRecord } from "@/domain/memory/api";
import type {
  MemoryMetadataItem,
  MemoryRecord,
} from "@/domain/memory/model/MemoryModel";
import type { MemoryMutationInput } from "@/domain/memory/components";

let memoryWorkDomains: MemoryWorkDomainRecord[] = [
  {
    createdAt: "2026-06-12 10:00",
    description: "UI implementation rules, design-system decisions, responsive behavior, and browser workflow memory.",
    domainCode: "frontend",
    name: "Frontend Engineering",
    status: "ACTIVE",
    updatedAt: "2026-06-17 09:20",
  },
  {
    createdAt: "2026-06-12 10:00",
    description: "Rust service contracts, migrations, storage behavior, and API integration memory.",
    domainCode: "backend",
    name: "Backend Engineering",
    status: "ACTIVE",
    updatedAt: "2026-06-16 14:10",
  },
  {
    createdAt: "2026-06-12 10:00",
    description: "Verification strategy, focused checks, Playwright scenarios, and release quality memory.",
    domainCode: "testing",
    name: "Testing",
    status: "ACTIVE",
    updatedAt: "2026-06-14 11:05",
  },
  {
    createdAt: "2026-06-12 10:00",
    description: "Product behavior, acceptance criteria, UX decisions, and rollout planning memory.",
    domainCode: "product",
    name: "Product",
    status: "ACTIVE",
    updatedAt: "2026-06-13 18:24",
  },
  {
    createdAt: "2026-06-12 10:00",
    description: "Default work-domain target used when a more specific domain is not selected.",
    domainCode: "main-domain",
    name: "Main Domain",
    status: "ACTIVE",
    updatedAt: "2026-06-12 12:30",
  },
  {
    createdAt: "2026-06-11 09:15",
    description: "Archived mobile-specific decisions retained for historical lookup only.",
    domainCode: "legacy-mobile",
    name: "Legacy Mobile",
    status: "DISABLED",
    updatedAt: "2026-06-08 16:42",
  },
];

let memoryRecords: MemoryRecord[] = [
  {
    id: "mem-space-response-format",
    injectable: true,
    memoryCode: "S:WS-633C49DF5B4CB335:api-response-format:v1",
    memoryStatus: "ACTIVE",
    memoryType: "SPACE_RULE",
    metadata: [
      { label: "Scope code", value: "WS-633C49DF5B4CB335" },
      { label: "Language", value: "zh-CN" },
      { label: "Source", value: "AGENTS.md" },
    ],
    reviewStatus: "APPROVED",
    scope: "space",
    score: 1,
    semanticKey: "space-rule:tron:api-response-format",
    sourceCode: "agents-md-api",
    sourceLabel: "Current space",
    tags: ["api", "contract", "pagination"],
    targetCode: "WS-633C49DF5B4CB335",
    targetId: "space:current",
    targetLabel: "long-term-memory-web",
    text: "返回体统一为 code、message、data；分页 data 包含 current、size、total、records。适用于接口文档和前后端联调。",
    title: "API 响应体必须使用统一格式",
    updatedAt: "2026-06-15 15:20",
  },
  {
    id: "mem-space-makefile-first",
    injectable: true,
    memoryCode: "S:WS-633C49DF5B4CB335:makefile-first:v1",
    memoryStatus: "ACTIVE",
    memoryType: "SPACE_RULE",
    metadata: [
      { label: "Scope code", value: "WS-633C49DF5B4CB335" },
      { label: "Validation", value: "make fe-check" },
      { label: "Source", value: "AGENTS.md" },
    ],
    reviewStatus: "APPROVED",
    scope: "space",
    score: 0.97,
    semanticKey: "space-rule:tron:makefile-first",
    sourceCode: "agents-md-build",
    sourceLabel: "Current space",
    tags: ["makefile", "build", "test"],
    targetCode: "WS-633C49DF5B4CB335",
    targetId: "space:current",
    targetLabel: "long-term-memory-web",
    text: "项目内构建、测试和清理优先使用 Makefile 入口。前端验证先走 make fe-check，再按需要启动本地 Vite 页面做浏览器检查。",
    title: "构建测试优先走 Makefile",
    updatedAt: "2026-06-15 14:52",
  },
  {
    id: "mem-space-ui-scope-tabs",
    injectable: false,
    memoryCode: "S:WS-633C49DF5B4CB335:memory-web-search-targets:v1",
    memoryStatus: "ACTIVE",
    memoryType: "PROJECT_DECISION",
    metadata: [
      { label: "Feature", value: "long-term-memory-web" },
      { label: "Default target", value: "space:current" },
      { label: "Design", value: "search-target-icons-v13" },
    ],
    reviewStatus: "APPROVED",
    scope: "space",
    score: 0.93,
    semanticKey: "project-decision:memory-web:search-target-navigation",
    sourceCode: "brainstorm-search-v13",
    sourceLabel: "Current space",
    tags: ["memory", "search", "navigation"],
    targetCode: "WS-633C49DF5B4CB335",
    targetId: "space:current",
    targetLabel: "long-term-memory-web",
    text: "记忆列表使用搜索结果式列表。空间、用户和领域作为搜索范围目标放在搜索框下方，默认选中当前空间，不提供 All 目标。",
    title: "记忆范围使用搜索目标导航",
    updatedAt: "2026-06-15 15:44",
  },
  {
    id: "mem-user-address",
    injectable: true,
    memoryCode: "U:USR-ADMIN:call-user-da-wang:v1",
    memoryStatus: "ACTIVE",
    memoryType: "USER_PREFERENCE",
    metadata: [
      { label: "Subject", value: "USR-ADMIN" },
      { label: "Language", value: "zh-CN" },
      { label: "Evidence", value: "User instruction" },
    ],
    reviewStatus: "APPROVED",
    scope: "user",
    score: 1,
    semanticKey: "user-preference:usr-admin:user-address:call-da-wang",
    sourceCode: "user-address-preference",
    sourceLabel: "Current user",
    tags: ["conversation", "address"],
    targetCode: "USR-ADMIN",
    targetId: "user:current",
    targetLabel: "USR-ADMIN",
    text: "后续与用户直接对话、确认需求或交付总结时，优先称呼用户为“大王”。技术上下文需要精确账号标识时仍使用正式主体。",
    title: "称呼用户为大王",
    updatedAt: "2026-06-05 11:28",
  },
  {
    id: "mem-user-assistant-name",
    injectable: true,
    memoryCode: "U:USR-ADMIN:assistant-name-duduwa:v1",
    memoryStatus: "ACTIVE",
    memoryType: "USER_PREFERENCE",
    metadata: [
      { label: "Subject", value: "USR-ADMIN" },
      { label: "Language", value: "zh-CN" },
      { label: "Evidence", value: "User instruction" },
    ],
    reviewStatus: "APPROVED",
    scope: "user",
    score: 0.96,
    semanticKey: "user-preference:usr-admin:assistant-name:duduwa",
    sourceCode: "assistant-name-preference",
    sourceLabel: "Current user",
    tags: ["conversation", "assistant-name"],
    targetCode: "USR-ADMIN",
    targetId: "user:current",
    targetLabel: "USR-ADMIN",
    text: "用户希望在自然对话中将助手称呼为“嘟嘟哇”。如果用户明确改用其他称呼，以最新要求为准。",
    title: "助手称呼偏好：嘟嘟哇",
    updatedAt: "2026-06-05 11:31",
  },
  {
    id: "mem-user-workflow",
    injectable: true,
    memoryCode: "U:USR-ADMIN:workflow-plan-before-edits:v1",
    memoryStatus: "ACTIVE",
    memoryType: "USER_PREFERENCE",
    metadata: [
      { label: "Subject", value: "USR-ADMIN" },
      { label: "Applies to", value: "Complex repo work" },
      { label: "Review", value: "Approved" },
    ],
    reviewStatus: "APPROVED",
    scope: "user",
    score: 0.94,
    semanticKey: "user-preference:usr-admin:collaboration:plan-before-edits",
    sourceCode: "collaboration-workflow-preference",
    sourceLabel: "Current user",
    tags: ["workflow", "collaboration", "repo"],
    targetCode: "USR-ADMIN",
    targetId: "user:current",
    targetLabel: "USR-ADMIN",
    text: "非简单开发和审查任务中，先说明计划、预计修改文件和验证方式。用户明确要求直接执行时，可以在保持边界清晰的前提下继续推进。",
    title: "复杂任务先说明计划与验证方式",
    updatedAt: "2026-06-05 12:03",
  },
  {
    domainCode: "frontend",
    id: "mem-frontend-responsive-governance",
    injectable: false,
    memoryCode: "D:frontend:memory-work-domain-responsive-governance:v1",
    memoryStatus: "ACTIVE",
    memoryType: "RISK",
    metadata: [
      { label: "Domain", value: "frontend" },
      { label: "Surface", value: "Memory governance UI" },
      { label: "Risk", value: "Mobile horizontal overflow" },
    ],
    reviewStatus: "PENDING_REVIEW",
    scope: "domain",
    score: 0.89,
    semanticKey: "risk:frontend:memory-ui:wide-table-on-small-screen",
    sourceCode: "memory-web-design-review",
    sourceLabel: "Work domain · frontend",
    tags: ["responsive", "table", "search-list"],
    targetCode: "frontend",
    targetId: "domain:frontend",
    targetLabel: "frontend",
    text: "小屏上不要使用宽表格承载治理字段。使用搜索结果式列表和详情页，避免表格列过多导致移动端横向滚动。",
    title: "小屏上不要使用宽表格承载治理字段",
    updatedAt: "2026-06-15 15:37",
  },
  {
    domainCode: "frontend",
    id: "mem-frontend-icon-targets",
    injectable: false,
    memoryCode: "D:frontend:memory-target-icons:v1",
    memoryStatus: "ACTIVE",
    memoryType: "WORK_DOMAIN_EXPERIENCE",
    metadata: [
      { label: "Domain", value: "frontend" },
      { label: "Pattern", value: "Icon plus label" },
      { label: "Fallback", value: "Tooltip title" },
    ],
    reviewStatus: "APPROVED",
    scope: "domain",
    score: 0.86,
    semanticKey: "work-domain-experience:frontend:target-icons-with-labels",
    sourceCode: "memory-web-design-review",
    sourceLabel: "Work domain · frontend",
    tags: ["icons", "navigation", "scope"],
    targetCode: "frontend",
    targetId: "domain:frontend",
    targetLabel: "frontend",
    text: "范围导航中去掉括号内的 space、user、domain 文案，改用图标表达类型。图标必须保留可访问名称或 tooltip，避免只靠视觉判断。",
    title: "搜索范围类型使用图标表达",
    updatedAt: "2026-06-15 15:41",
  },
  {
    domainCode: "backend",
    id: "mem-backend-work-domain-validation",
    injectable: false,
    memoryCode: "D:backend:memory-work-domain-validation:v1",
    memoryStatus: "CANDIDATE",
    memoryType: "PROCEDURE",
    metadata: [
      { label: "Domain", value: "backend" },
      { label: "Scope", value: "WORK_DOMAIN" },
      { label: "Status", value: "Needs API design" },
    ],
    reviewStatus: "PENDING_REVIEW",
    scope: "domain",
    score: 0.78,
    semanticKey: "procedure:backend:memory-work-domain-validation",
    sourceCode: "memory-service-analysis",
    sourceLabel: "Work domain · backend",
    tags: ["work-domain", "validation", "api"],
    targetCode: "backend",
    targetId: "domain:backend",
    targetLabel: "backend",
    text: "长期记忆后端需要明确可用工作领域列表。前端不能把 agent tool domain 当作长期记忆 work domain，否则保存和查询会出现 scopeCode 不可验证的问题。",
    title: "工作领域需要独立的可选列表",
    updatedAt: "2026-06-15 15:09",
  },
  {
    domainCode: "backend",
    id: "mem-backend-memory-service-wrapper",
    injectable: false,
    memoryCode: "D:backend:memory-service-wrapper:v1",
    memoryStatus: "ACTIVE",
    memoryType: "FACT",
    metadata: [
      { label: "Domain", value: "backend" },
      { label: "Module", value: "MemoryService" },
      { label: "Storage", value: "RAG memory-card corpus" },
    ],
    reviewStatus: "APPROVED",
    scope: "domain",
    score: 0.84,
    semanticKey: "fact:backend:memory-service-wraps-rag",
    sourceCode: "memory-service-analysis",
    sourceLabel: "Work domain · backend",
    tags: ["memory-service", "rag", "tools"],
    targetCode: "backend",
    targetId: "domain:backend",
    targetLabel: "backend",
    text: "长期记忆当前由 MemoryService 包装 RAG memory-card corpus，并通过 memory.search、memory.save、memory.delete 三个 business tools 暴露给 agent。",
    title: "长期记忆服务包装 RAG 记忆卡片",
    updatedAt: "2026-06-15 14:58",
  },
  {
    domainCode: "testing",
    id: "mem-testing-browser-check",
    injectable: false,
    memoryCode: "D:testing:memory-web-browser-check:v1",
    memoryStatus: "ACTIVE",
    memoryType: "PROCEDURE",
    metadata: [
      { label: "Domain", value: "testing" },
      { label: "Tool", value: "Playwright" },
      { label: "Viewports", value: "Desktop and mobile" },
    ],
    reviewStatus: "APPROVED",
    scope: "domain",
    score: 0.88,
    semanticKey: "procedure:testing:memory-web-browser-visual-check",
    sourceCode: "memory-web-visual-companion",
    sourceLabel: "Work domain · testing",
    tags: ["playwright", "screenshot", "responsive"],
    targetCode: "testing",
    targetId: "domain:testing",
    targetLabel: "testing",
    text: "记忆列表页面完成后需要用浏览器检查桌面和移动端，重点确认搜索框、范围导航、More/Filters 菜单和结果文本没有重叠或不可读。",
    title: "记忆页面需做浏览器视觉检查",
    updatedAt: "2026-06-15 15:45",
  },
  {
    domainCode: "product",
    id: "mem-product-default-scope",
    injectable: false,
    memoryCode: "D:product:memory-web-default-scope:v1",
    memoryStatus: "ACTIVE",
    memoryType: "PROJECT_DECISION",
    metadata: [
      { label: "Domain", value: "product" },
      { label: "Default", value: "space:current" },
      { label: "Excluded", value: "All target" },
    ],
    reviewStatus: "APPROVED",
    scope: "domain",
    score: 0.91,
    semanticKey: "project-decision:product:memory-web-default-space",
    sourceCode: "memory-web-design-review",
    sourceLabel: "Work domain · product",
    tags: ["scope", "space", "search"],
    targetCode: "product",
    targetId: "domain:product",
    targetLabel: "product",
    text: "用户进入页面时已经处在当前空间维度，因此默认搜索当前空间。不提供 All 范围，用户可以显式切换到当前用户或某个领域。",
    title: "记忆列表默认搜索当前空间",
    updatedAt: "2026-06-15 15:32",
  },
];

function formatTimestamp(date = new Date()) {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function slugValue(value: string) {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "memory";
}

function sourceCodeFromLabel(value: string) {
  return slugValue(value || "manual-entry");
}

function semanticKeyFromInput(input: MemoryMutationInput) {
  if (input.semanticKey.trim()) return input.semanticKey.trim();
  return `${input.memoryType.toLowerCase()}:${input.scope}:${slugValue(input.title)}`;
}

function memoryCodePrefix(scope: MemoryMutationInput["scope"]) {
  if (scope === "space") return "S";
  if (scope === "user") return "U";
  return "D";
}

function memoryCodeFromInput(input: MemoryMutationInput, semanticKey: string) {
  return `${memoryCodePrefix(input.scope)}:${input.targetCode}:${slugValue(semanticKey)}:v1`;
}

function normalizeMetadata(metadata: MemoryMetadataItem[]) {
  return metadata
    .map((item) => ({
      label: item.label.trim(),
      value: item.value.trim(),
    }))
    .filter((item) => item.label || item.value);
}

function recordFromInput(input: MemoryMutationInput): MemoryRecord {
  const semanticKey = semanticKeyFromInput(input);
  return {
    domainCode: input.domainCode,
    id: `mem-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
    injectable: input.injectable,
    memoryCode: memoryCodeFromInput(input, semanticKey),
    memoryStatus: input.memoryStatus,
    memoryType: input.memoryType,
    metadata: normalizeMetadata(input.metadata),
    reviewStatus: input.reviewStatus,
    scope: input.scope,
    score: 1,
    semanticKey,
    sourceCode: sourceCodeFromLabel(input.sourceLabel),
    sourceLabel: input.sourceLabel,
    tags: input.tags,
    targetCode: input.targetCode,
    targetId: input.targetId,
    targetLabel: input.targetLabel,
    text: input.text,
    title: input.title,
    updatedAt: formatTimestamp(),
  };
}

export function getMemoryFragments(): MemoryRecord[] {
  return [...memoryRecords];
}

export function listMemoryWorkDomainsMock(options: { includeDisabled?: boolean; search?: string } = {}) {
  const includeDisabled = options.includeDisabled ?? false;
  const normalizedQuery = (options.search ?? "").trim().toLowerCase();

  return memoryWorkDomains.filter((domain) => {
    if (!includeDisabled && domain.status === "DISABLED") return false;
    if (!normalizedQuery) return true;

    return [
      domain.domainCode,
      domain.name,
      domain.description ?? "",
      domain.status,
    ].some((value) => value.toLowerCase().includes(normalizedQuery));
  });
}

export function getMemoryFragment(memoryId: string): MemoryRecord | null {
  return memoryRecords.find((memory) => memory.id === memoryId) ?? null;
}

export function createMemoryFragment(input: MemoryMutationInput): MemoryRecord {
  const record = recordFromInput(input);
  memoryRecords = [record, ...memoryRecords];
  return record;
}

export function updateMemoryFragment(memoryId: string, input: MemoryMutationInput): MemoryRecord {
  const current = getMemoryFragment(memoryId);
  if (!current) {
    throw new Error("Memory not found");
  }

  const next: MemoryRecord = {
    ...current,
    domainCode: input.domainCode,
    injectable: input.injectable,
    memoryStatus: input.memoryStatus,
    memoryType: input.memoryType,
    metadata: normalizeMetadata(input.metadata),
    reviewStatus: input.reviewStatus,
    semanticKey: input.semanticKey.trim() || current.semanticKey,
    sourceLabel: input.sourceLabel,
    tags: input.tags,
    text: input.text,
    title: input.title,
    updatedAt: formatTimestamp(),
  };
  memoryRecords = memoryRecords.map((record) => (record.id === memoryId ? next : record));
  return next;
}

export function archiveMemoryFragment(memoryId: string): MemoryRecord {
  const current = getMemoryFragment(memoryId);
  if (!current) {
    throw new Error("Memory not found");
  }
  const next = {
    ...current,
    memoryStatus: "ARCHIVED" as const,
    updatedAt: formatTimestamp(),
  };
  memoryRecords = memoryRecords.map((record) => (record.id === memoryId ? next : record));
  return next;
}

export function disableMemoryFragment(memoryId: string): MemoryRecord {
  const current = getMemoryFragment(memoryId);
  if (!current) {
    throw new Error("Memory not found");
  }
  const next = {
    ...current,
    injectable: false,
    memoryStatus: "DISABLED" as const,
    updatedAt: formatTimestamp(),
  };
  memoryRecords = memoryRecords.map((record) => (record.id === memoryId ? next : record));
  return next;
}

export function createMemoryWorkDomainRecord(input: {
  description?: string | null;
  domainCode: string;
  name: string;
}) {
  const domainCode = input.domainCode.trim();
  if (memoryWorkDomains.some((domain) => domain.domainCode === domainCode)) {
    throw new Error("Memory domain already exists");
  }

  const record: MemoryWorkDomainRecord = {
    createdAt: formatTimestamp(),
    description: input.description?.trim() || null,
    domainCode,
    name: input.name.trim(),
    status: "ACTIVE",
    updatedAt: formatTimestamp(),
  };

  memoryWorkDomains = [record, ...memoryWorkDomains];
  return record;
}

export function updateMemoryWorkDomainRecord(domainCode: string, input: {
  description?: string | null;
  name?: string;
  status?: MemoryWorkDomainRecord["status"];
}) {
  const current = memoryWorkDomains.find((domain) => domain.domainCode === domainCode);
  if (!current) {
    throw new Error("Memory domain not found");
  }

  const next: MemoryWorkDomainRecord = {
    ...current,
    description: input.description === undefined ? current.description : (input.description?.trim() || null),
    name: input.name?.trim() || current.name,
    status: input.status ?? current.status,
    updatedAt: formatTimestamp(),
  };

  memoryWorkDomains = memoryWorkDomains.map((domain) => (
    domain.domainCode === domainCode ? next : domain
  ));
  return next;
}

export function enableMemoryWorkDomainRecord(domainCode: string) {
  return updateMemoryWorkDomainRecord(domainCode, { status: "ACTIVE" });
}

export function disableMemoryWorkDomainRecord(domainCode: string) {
  return updateMemoryWorkDomainRecord(domainCode, { status: "DISABLED" });
}
