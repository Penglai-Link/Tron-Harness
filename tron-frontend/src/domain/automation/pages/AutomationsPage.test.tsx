// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import { createInstance } from "i18next";
import { I18nextProvider } from "react-i18next";

// The route must not initialize the shell, chat terminal or embedded browser.
vi.mock("@/shared/layout/Shell", () => {
  throw new Error("Automation route pulled in the application shell");
});
afterEach(cleanup);
it("loads the automation page and renders without waiting for list data", async () => {
  const { AutomationsPage } = await import("./AutomationsPage");
  const i18n = createInstance();
  await i18n.init({ lng: "en-US", resources: { "en-US": { common: {}, pages: {} } } });
  render(<I18nextProvider i18n={i18n}><MemoryRouter><AutomationsPage /></MemoryRouter></I18nextProvider>);
  expect(screen.getByRole("button", { name: "New" }).hasAttribute("disabled")).toBe(true);
  expect(screen.getByText("Choose a workspace in the shell header before managing automations.")).toBeTruthy();
}, 15000);
