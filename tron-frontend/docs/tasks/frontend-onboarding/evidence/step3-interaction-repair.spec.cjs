const { test, expect } = require("@playwright/test");

const baseUrl = "http://127.0.0.1:4173";

const repairedPages = [
  ["audit_logs_tron_standardized_navigation", "/admin/audit-logs"],
  ["automations_management_light", "/automations"],
  ["create_new_prompt_tron", "/prompts/new"],
  ["knowledge_graph_tron", "/knowledge-graph"],
  ["knowledge_graph_tron_domain_filtering", "/knowledge-graph/domain-filtering"],
  ["main_agent_chat_tron", "/agent"],
  ["memory_management_tron_data_isolation_1", "/memory"],
  ["memory_management_tron_data_isolation_2", "/memory/details"],
  ["model_registry_tron", "/models"],
  ["role_matrix_tron_standardized", "/admin/roles"],
  ["skills_mcp_management_tron", "/skills"],
  ["task_details_lumina_workspace_updated_hierarchy", "/tasks/detail"],
  ["task_management_light", "/tasks"],
  ["user_permissions_tron", "/admin/users"],
  ["variable_management_tron_standardized_1", "/variables"],
  ["variable_management_tron_standardized_2", "/variables/details"],
];

async function gotoAndCapture(page, prototype, route) {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto(`${baseUrl}${route}`, { waitUntil: "networkidle" });
  await expect(page.locator("body")).toBeVisible();
  await page.screenshot({ path: `docs/tasks/frontend-onboarding/evidence/step3-current/${prototype}.png`, fullPage: false });
}

test("Step 3 repaired pages expose observable action behavior", async ({ page }) => {
  for (const [prototype, route] of repairedPages) {
    await gotoAndCapture(page, prototype, route);
  }

  await page.goto(`${baseUrl}/admin/audit-logs`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Export" }).click();
  await expect(page.getByText(/audit rows exported/)).toBeVisible();

  await page.goto(`${baseUrl}/automations`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "New Automation" }).click();
  await expect(page.getByText("New automation draft opened")).toBeVisible();
  await page.getByRole("button", { name: "Filter", exact: true }).click();
  await expect(page.getByText(/Filter panel opened/)).toBeVisible();

  await page.goto(`${baseUrl}/prompts/new`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "New Project" }).click();
  await expect(page.getByText(/New project draft opened/)).toBeVisible();

  await page.goto(`${baseUrl}/knowledge-graph`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Edit Entity" }).click();
  await expect(page.getByText(/Editing /)).toBeVisible();

  await page.goto(`${baseUrl}/knowledge-graph/domain-filtering`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Edit Entity" }).click();
  await expect(page.getByText(/Editing /)).toBeVisible();

  await page.goto(`${baseUrl}/agent`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Open attachment picker" }).click();
  await expect(page.getByText("Attachment picker opened")).toBeVisible();

  await page.goto(`${baseUrl}/memory`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Import Data" }).click();
  await expect(page.getByText("Import data drawer opened")).toBeVisible();
  await page.getByRole("button", { name: "Sync Indices" }).click();
  await expect(page.getByText("Index sync queued")).toBeVisible();

  await page.goto(`${baseUrl}/memory/details`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Import Data" }).click();
  await expect(page.getByText("Import data drawer opened")).toBeVisible();
  await page.getByRole("button", { name: "Sync Indices" }).click();
  await expect(page.getByText("Index sync queued")).toBeVisible();

  await page.goto(`${baseUrl}/models`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Add Model" }).click();
  await expect(page.getByText("Add model workflow opened")).toBeVisible();
  await page.getByRole("button", { name: "Open gpt-4-turbo settings" }).click();
  await expect(page.getByText("Panel state: Settings opened")).toBeVisible();

  await page.goto(`${baseUrl}/admin/roles`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Add Custom Role" }).click();
  await expect(page.getByText("Custom role editor opened")).toBeVisible();
  await page.getByRole("button", { name: "Filter", exact: true }).click();
  await expect(page.getByText(/Filter drawer opened/)).toBeVisible();

  await page.goto(`${baseUrl}/skills`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "New Skill" }).click();
  await expect(page.getByText("New skill workflow opened")).toBeVisible();

  await page.goto(`${baseUrl}/tasks/detail`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Execute Task" }).click();
  await expect(page.getByText("Task execution queued")).toBeVisible();
  await page.getByRole("button", { name: "Add Sub-task" }).click();
  await expect(page.getByText(/Sub-task \d+ added/)).toBeVisible();

  await page.goto(`${baseUrl}/tasks`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Sync" }).click();
  await expect(page.getByText("Kanban sync completed")).toBeVisible();
  await page.getByRole("button", { name: "Create Task" }).click();
  await expect(page.getByText("Create task workflow opened")).toBeVisible();

  await page.goto(`${baseUrl}/admin/users`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Export" }).click();
  await expect(page.getByText(/users exported/)).toBeVisible();
  await page.getByRole("button", { name: "Add User" }).click();
  await expect(page.getByText("Add user drawer opened")).toBeVisible();
  await page.getByRole("button", { name: "Open actions for Alex Johnson" }).click();
  await expect(page.getByText("Alex Johnson action menu opened")).toBeVisible();
  await page.locator("tbody select").first().selectOption("Viewer");
  await expect(page.getByText("Role updated to Viewer")).toBeVisible();

  await page.goto(`${baseUrl}/variables`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "New Variable" }).click();
  await expect(page.getByText("New variable editor opened")).toBeVisible();
  await page.getByRole("button", { name: /Delete / }).first().click();
  await expect(page.getByText(/Delete .* requested/)).toBeVisible();

  await page.goto(`${baseUrl}/variables/details`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "New Variable" }).click();
  await expect(page.getByText("New variable editor opened")).toBeVisible();
  await page.getByRole("button", { name: /Delete / }).first().click();
  await expect(page.getByText(/Delete .* requested/)).toBeVisible();
});
