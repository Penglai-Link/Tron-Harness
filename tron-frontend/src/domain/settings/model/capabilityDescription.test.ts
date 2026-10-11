import { createInstance } from "i18next";
import { expect, it } from "vitest";
import { pages as en } from "@/i18n/locales/en-US/pages";
import { pages as zh } from "@/i18n/locales/zh-CN/pages";
import { capabilityDescription } from "./capabilityDescription";

it("localizes built-in pack, plugin and runtime descriptions while preserving external metadata", async () => {
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", resources: { "zh-CN": { pages: zh }, "en-US": { pages: en } } });
  expect(capabilityDescription(i18n.t, "Pack", "framework.external-agent", "original")).toBe("外部 Agent 调用与会话衔接");
  expect(capabilityDescription(i18n.t, "Plugin", "framework.loop", "original")).toContain("原生会话归属");
  expect(capabilityDescription(i18n.t, "Implementation", "HostExternalAgent", "original")).toContain("宿主适配器");
  expect(capabilityDescription(i18n.t, "Pack", "custom.pack", "Custom description")).toBe("Custom description");
  await i18n.changeLanguage("en-US");
  expect(capabilityDescription(i18n.t, "Pack", "framework.external-agent", "original")).toBe("External agent invocation and session bridging");
});
