import type { TFunction } from "i18next";
import type { LoopDesign, LoopNode } from "./LoopModel";

export const DEFAULT_REACT_LOOP_CODE = "default-react";

export function isBuiltinLoop(code: string): boolean {
  return code === DEFAULT_REACT_LOOP_CODE;
}
export function loopName(design: Pick<LoopDesign, "loopCode" | "name">, _t?: TFunction): string {
  if (isBuiltinLoop(design.loopCode)) return "EliteForge ReAct";
  return design.name;
}
export function loopDescription(design: LoopDesign, t?: TFunction): string | null {
  if (isBuiltinLoop(design.loopCode)) return t?.("agentLoops.reactHelp") ?? null;
  return null;
}
export function loopProvider(design: Pick<LoopDesign, "loopCode">): { provider: string; pluginId: string } | null {
  return isBuiltinLoop(design.loopCode)
    ? { provider: "EliteForge", pluginId: "framework.react" }
    : null;
}
export function loopNodeName(_design: LoopDesign, node: LoopNode, _t?: TFunction): string {
  return node.id;
}
export function copyLoopDesign(design: LoopDesign, name: string): LoopDesign {
  return {
    ...structuredClone(design),
    loopCode: "",
    name,
    version: 0,
    updatedAt: 0,
  };
}
