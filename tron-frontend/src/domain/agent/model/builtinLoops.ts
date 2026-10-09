import type { TFunction } from "i18next";
import catalog from "../../../../../shared/agent-loops.json";
import type { LoopDesign, LoopNode } from "./LoopModel";

export function builtinLoopDesigns(): LoopDesign[] {
  return structuredClone(catalog) as LoopDesign[];
}
export function isBuiltinLoop(code: string): boolean {
  return catalog.some((design) => design.loopCode === code);
}
const keys = new Map([
  ["builtin.review-workflow", "review"],
  ["builtin.analysis-dag", "analysis"],
]);
export function loopName(
  design: Pick<LoopDesign, "loopCode" | "name">,
  t: TFunction,
): string {
  const key = keys.get(design.loopCode);
  return key ? t(`agentLoops.builtins.${key}.name`) : design.name;
}
export function loopDescription(
  design: LoopDesign,
  t: TFunction,
): string | null {
  const key = keys.get(design.loopCode);
  return key ? t(`agentLoops.builtins.${key}.description`) : null;
}
export function loopNodeName(
  design: LoopDesign,
  node: LoopNode,
  t: TFunction,
): string {
  const key = keys.get(design.loopCode);
  return key
    ? t(`agentLoops.builtins.${key}.nodes.${node.id}`, {
        defaultValue: node.id,
      })
    : node.id;
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
