const { test, expect } = require("@playwright/test");

test("Step 3 shell keeps a full-width topbar, below-topbar sidebar, and collapse state", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto("http://127.0.0.1:4173/skills", { waitUntil: "networkidle" });

  const topbar = page.locator("header").first();
  const sidebar = page.locator("aside[data-collapsed]").first();

  await expect(topbar).toHaveCSS("position", "fixed");
  await expect(sidebar).toHaveAttribute("data-collapsed", "false");

  const topbarBox = await topbar.boundingBox();
  const sidebarBox = await sidebar.boundingBox();

  expect(topbarBox).not.toBeNull();
  expect(sidebarBox).not.toBeNull();
  expect(Math.round(topbarBox.x)).toBe(0);
  expect(Math.round(topbarBox.y)).toBe(0);
  expect(Math.round(topbarBox.width)).toBe(1440);
  expect(Math.round(topbarBox.height)).toBe(56);
  expect(Math.round(sidebarBox.y)).toBe(56);
  expect(Math.round(sidebarBox.width)).toBe(280);

  await page.screenshot({ path: "docs/tasks/frontend-onboarding/evidence/step3-shell-repair-skills-expanded.png" });

  await page.getByRole("button", { name: "Collapse sidebar" }).click();
  await expect(sidebar).toHaveAttribute("data-collapsed", "true");
  await expect(page.getByRole("button", { name: "Expand sidebar" })).toBeVisible();
  await page.waitForTimeout(250);

  const collapsedSidebarBox = await sidebar.boundingBox();
  expect(collapsedSidebarBox).not.toBeNull();
  expect(Math.round(collapsedSidebarBox.width)).toBe(76);

  await page.screenshot({ path: "docs/tasks/frontend-onboarding/evidence/step3-shell-repair-skills-collapsed.png" });
});
