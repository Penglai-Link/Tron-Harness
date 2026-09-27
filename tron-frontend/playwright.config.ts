import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  testMatch: "**/*.spec.ts",
  use: {
    channel: process.env.PLAYWRIGHT_CHANNEL || undefined,
  },
});
