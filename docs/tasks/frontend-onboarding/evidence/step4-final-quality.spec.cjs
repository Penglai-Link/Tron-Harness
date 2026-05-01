const fs = require("fs");
const path = require("path");
const { test, expect } = require("@playwright/test");

const baseUrl = process.env.TRON_BASE_URL || "http://127.0.0.1:4173";
const screenshotDir = path.join(process.cwd(), "docs/tasks/frontend-onboarding/evidence/step4-current");

const pages = [
  {
    prototype: "add_model_tron_standardized_navigation",
    route: "/models/new",
    shell: "standard",
    markers: ["Configure Model", "Provider Details", "Execution Parameters", "Configuration Preview"],
  },
  {
    prototype: "app_management_light",
    route: "/apps",
    shell: "standard",
    markers: ["Connected Applications", "Application Registry", "App Name", "Worktree"],
  },
  {
    prototype: "audit_logs_tron_standardized_navigation",
    route: "/admin/audit-logs",
    shell: "standard",
    markers: ["Users & Roles", "Audit Logs", "Timestamp", "IP Address"],
  },
  {
    prototype: "automations_management_light",
    route: "/automations",
    shell: "standard",
    markers: ["Automations Management", "Dependency Flow", "Details", "Logs", "Traces"],
  },
  {
    prototype: "chat_sessions_updated_tron_1",
    route: "/chats/8842-alpha",
    shell: "standard",
    markers: ["Sessions", "Primary Investigation", "Reasoning view", "Utility Hub"],
  },
  {
    prototype: "chat_sessions_updated_tron_2",
    route: "/chats/8842-alpha?variant=execution",
    shell: "standard",
    markers: ["Sessions", "Primary Investigation", "Execution view", "Utility Hub"],
  },
  {
    prototype: "create_automation_tron_standardized",
    route: "/automations/new",
    shell: "standard",
    markers: ["Create Automation", "Trigger Configuration", "Application & Agent", "Dependencies"],
  },
  {
    prototype: "create_new_prompt_tron",
    route: "/prompts/new",
    shell: "prompt",
    markers: ["Tron AI", "Create New Prompt", "Prompt Template", "Detected Variables"],
  },
  {
    prototype: "create_new_skill_tron_standardized",
    route: "/skills/new",
    shell: "standard",
    markers: ["Create New Skill", "General Details", "Tool Definition", "Schema Preview"],
  },
  {
    prototype: "create_task_full_width_console_layout",
    route: "/tasks/new",
    shell: "standard",
    markers: ["Create Task", "Assign Agent", "Description", "Quick Reference"],
  },
  {
    prototype: "dashboard_light",
    route: "/dashboard",
    shell: "standard",
    markers: ["Workspace Overview", "Inheritance Map", "Recent Activity"],
  },
  {
    prototype: "import_application_tron_unified_navigation_1",
    route: "/apps/import?source=github",
    shell: "standard",
    markers: ["Import Application", "Source", "Repository URL", "Branch / Ref"],
  },
  {
    prototype: "import_application_tron_unified_navigation_2",
    route: "/apps/import?source=local",
    shell: "standard",
    markers: ["Import Application", "Source", "Local File System Path", "App Name"],
  },
  {
    prototype: "knowledge_graph_tron",
    route: "/knowledge-graph",
    shell: "standard",
    markers: ["Entities", "Zoom 100%", "Properties", "Relationships"],
  },
  {
    prototype: "knowledge_graph_tron_domain_filtering",
    route: "/knowledge-graph/domain-filtering",
    shell: "standard",
    markers: ["Entities", "Domain", "Domain: Customer Support", "Relationships"],
  },
  {
    prototype: "login_tron_light_theme_centered",
    route: "/login",
    shell: "none",
    markers: ["Tron", "Operator ID", "Passkey", "Captcha", "Sign In"],
  },
  {
    prototype: "main_agent_chat_tron",
    route: "/agent",
    shell: "standard",
    markers: ["Main Agent", "Reasoning Process", "FetchClusterMetrics", "Execute Scale-up"],
  },
  {
    prototype: "memory_management_tron_data_isolation_1",
    route: "/memory",
    shell: "standard",
    markers: ["Memory Repository", "Memory Fragments", "Active Filters", "Confidence"],
  },
  {
    prototype: "memory_management_tron_data_isolation_2",
    route: "/memory/details",
    shell: "standard",
    markers: ["Memory Repository", "Memory Details", "Edit Fragment", "Invalidate"],
  },
  {
    prototype: "model_registry_tron",
    route: "/models",
    shell: "standard",
    markers: ["Model Registry", "Model Name", "Configuration Details", "Performance Benchmarks"],
  },
  {
    prototype: "observability_dashboard_light",
    route: "/observability",
    shell: "standard",
    markers: ["Observability Dashboard", "Request Throughput", "Error Heatmap", "Recent Traces"],
  },
  {
    prototype: "prompt_management_tron_standardized",
    route: "/prompts",
    shell: "standard",
    markers: ["Prompt Management", "All Prompts", "Last Updated", "Test Variables"],
  },
  {
    prototype: "resources_management_tron",
    route: "/resources",
    shell: "enterprise",
    markers: ["Tron Console", "Enterprise Management", "Data & Assets", "Resource Name", "Active Data Sources"],
  },
  {
    prototype: "role_matrix_tron_standardized",
    route: "/admin/roles",
    shell: "standard",
    markers: ["Platform Permissions", "Role Matrix", "Workspace", "MCP"],
  },
  {
    prototype: "skills_mcp_management_tron",
    route: "/skills",
    shell: "standard",
    markers: ["Capabilities Management", "Skills", "MCP Servers", "Tool Definition (Schema)"],
  },
  {
    prototype: "task_details_lumina_workspace_updated_hierarchy",
    route: "/tasks/detail",
    shell: "standard",
    markers: ["Task Details", "Description & Requirements", "Execution Timeline", "Sub-tasks"],
  },
  {
    prototype: "task_management_light",
    route: "/tasks",
    shell: "standard",
    markers: ["Task Management", "Backlog", "To Do", "In Progress", "Done"],
  },
  {
    prototype: "user_permissions_tron",
    route: "/admin/users",
    shell: "standard",
    markers: ["Users & Roles", "User Directory", "Role Matrix", "Last Active"],
  },
  {
    prototype: "variable_management_tron_standardized_1",
    route: "/variables",
    shell: "standard",
    markers: ["Variable inspector ready", "Key", "Usage in Prompts", "Reveal"],
  },
  {
    prototype: "variable_management_tron_standardized_2",
    route: "/variables/details",
    shell: "standard",
    markers: ["Variable Management", "Key", "Usage in Prompts", "Reveal"],
  },
];

test.describe.configure({ mode: "serial" });

test.beforeAll(() => {
  fs.mkdirSync(screenshotDir, { recursive: true });
});

for (const item of pages) {
  test(`Step 4 structure and screenshot evidence: ${item.prototype}`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1100 });
    await page.goto(`${baseUrl}${item.route}`, { waitUntil: "networkidle" });
    await expect(page.locator("body")).toBeVisible();

    for (const marker of item.markers) {
      await expect(page.locator("body")).toContainText(marker);
    }

    if (item.shell === "standard") {
      await expect(page.locator("header").first()).toBeVisible();
      await expect(page.locator("aside[data-collapsed]").first()).toBeVisible();
      await expect(page.getByRole("button", { name: "Collapse sidebar" })).toBeVisible();
    } else if (item.shell === "prompt") {
      await expect(page.locator("aside").first()).toContainText("Tron AI");
      await expect(page.locator("header").first()).toContainText("Dashboard");
    } else if (item.shell === "enterprise") {
      await expect(page.locator("aside").first()).toContainText("Enterprise Management");
      await expect(page.locator("header").first()).toContainText("Resources Management");
    } else {
      await expect(page.locator("aside[data-collapsed]")).toHaveCount(0);
    }

    await page.screenshot({ path: path.join(screenshotDir, `${item.prototype}.png`), fullPage: false });
  });
}

test("Step 4 standard shell and skills interactions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });

  await page.goto(`${baseUrl}/skills`, { waitUntil: "networkidle" });
  const sidebar = page.locator("aside[data-collapsed]").first();
  await expect(sidebar).toHaveAttribute("data-collapsed", "false");
  await page.getByRole("button", { name: "Collapse sidebar" }).click();
  await expect(sidebar).toHaveAttribute("data-collapsed", "true");
  await page.getByRole("button", { name: "Expand sidebar" }).click();
  await expect(sidebar).toHaveAttribute("data-collapsed", "false");
  const mcpTab = page.locator("button").filter({ hasText: "MCP Servers" }).first();
  await mcpTab.click();
  await expect(mcpTab).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "New Skill" }).click();
  await expect(page.getByText("New skill workflow opened")).toBeVisible();
});

test("Step 4 kanban interactions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto(`${baseUrl}/tasks`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Sync" }).click();
  await expect(page.getByText("Kanban sync completed")).toBeVisible();
  await page.getByRole("button", { name: "Create Task" }).click();
  await expect(page.getByText("Create task workflow opened")).toBeVisible();
  await page.locator("button").filter({ hasText: /AGT-\d+/ }).first().click();
  await expect(page.getByText(/AGT-\d+/).first()).toBeVisible();
});

test("Step 4 memory interactions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto(`${baseUrl}/memory`, { waitUntil: "networkidle" });
  const semanticTab = page.locator("button").filter({ hasText: "Semantic Only" }).first();
  await semanticTab.click();
  await expect(semanticTab).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "Add Filter" }).click();
  await expect(page.getByText("Strategy: semantic")).toBeVisible();
});

test("Step 4 resources interactions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto(`${baseUrl}/resources`, { waitUntil: "networkidle" });
  await page.getByPlaceholder("Search resources...").fill("s3");
  await expect(page.getByText("Search filtered to s3")).toBeVisible();
  await page.getByRole("button", { name: /Type:/ }).click();
  await expect(page.getByText(/Type filter changed to/)).toBeVisible();
  await page.getByLabel("Select all resources").click();
  await expect(page.getByText("Visible resources selected")).toBeVisible();
});

test("Step 4 prompt-shell interactions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto(`${baseUrl}/prompts/new`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Deployments" }).click();
  await expect(page.getByRole("button", { name: "Deployments" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Notifications" }).click();
  await expect(page.getByText("3 prompt workflow alerts are ready for review.")).toBeVisible();
});

test("Step 4 auth interactions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto(`${baseUrl}/login`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Refresh" }).click();
  await expect(page.getByText("M2R8")).toBeVisible();
});
