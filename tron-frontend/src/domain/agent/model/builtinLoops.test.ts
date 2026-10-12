import { expect, it } from "vitest";
import { isBuiltinLoop, loopName, loopProvider } from "./builtinLoops";

it("identifies the protected default ReAct Loop and its plugin provider", () => {
  const builtin = { loopCode: "default-react", name: "默认 ReAct" };
  expect(isBuiltinLoop(builtin.loopCode)).toBe(true);
  expect(loopName(builtin)).toBe("EliteForge ReAct");
  expect(loopProvider(builtin)).toEqual({ provider: "EliteForge", pluginId: "framework.react" });
  expect(isBuiltinLoop("custom-loop")).toBe(false);
});
