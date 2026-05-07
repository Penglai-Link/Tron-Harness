const { test, expect } = require("@playwright/test");

test("Step 3 prompt shell navigation and notifications expose observable behavior", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto("http://127.0.0.1:4173/prompts/new", { waitUntil: "networkidle" });

  await expect(page.getByRole("button", { name: "Library" })).toHaveAttribute("aria-pressed", "true");

  await page.getByRole("button", { name: "Deployments" }).click();
  await expect(page.getByRole("button", { name: "Deployments" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByText(/Deployments navigation selected/)).toBeVisible();

  await page.getByRole("button", { name: "Notifications" }).click();
  await expect(page.getByText("3 prompt workflow alerts are ready for review.")).toBeVisible();
  await expect(page.getByText(/Notifications panel opened/)).toBeVisible();

  await page.getByRole("button", { name: "Open center" }).click();
  await expect(page.getByText("Notification center opened")).toBeVisible();

  await page.screenshot({ path: "docs/tasks/frontend-onboarding/evidence/step3-current/create_new_prompt_tron.png", fullPage: false });
});
