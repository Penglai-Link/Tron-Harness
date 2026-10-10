import type { TFunction } from "i18next";
import type { LoopDesign, LoopNode } from "./LoopModel";

export function isBuiltinLoop(_code: string): boolean { return false; }
export function loopName(design: Pick<LoopDesign, "name">, _t?: TFunction): string {
  return design.name;
}
export function loopDescription(_design: LoopDesign, _t?: TFunction): string | null {
  return null;
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
