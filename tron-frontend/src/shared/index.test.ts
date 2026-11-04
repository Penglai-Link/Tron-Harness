// @vitest-environment jsdom
import { expect, it, vi } from "vitest";

vi.mock("./layout/Shell", () => {
  throw new Error("Shared utilities must not initialize the application shell");
});
vi.mock("./layout/SettingsShell", () => {
  throw new Error("Shared utilities must not initialize the settings shell");
});

it("loads shared utilities without pulling in application shells", async () => {
  const shared = await import("./index");
  expect(shared.SurfaceCard).toBeTypeOf("function");
  expect(shared.useOpenRightSidebar).toBeTypeOf("function");
});
