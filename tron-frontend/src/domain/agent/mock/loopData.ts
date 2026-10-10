import type { LoopDesign } from "../model/LoopModel";
import { isLoopUsedByAgentMock } from "./managementData";
export class LoopDesignMockError extends Error {
  constructor(public readonly reason: "conflict" | "inUse" | "notFound") {
    super(reason);
  }
}
let designs: LoopDesign[] = [];
export function listLoopDesignsMock() {
  return structuredClone(designs);
}
export function saveLoopDesignMock(design: LoopDesign) {
  const previous = designs.find((d) => d.loopCode === design.loopCode);
  if (
    (previous?.version ?? 0) !== design.version
  )
    throw new LoopDesignMockError("conflict");
  const saved = structuredClone({
    ...design,
    version: design.version + 1,
    updatedAt: Date.now(),
  });
  designs = [...designs.filter((d) => d.loopCode !== design.loopCode), saved];
  return saved;
}
export function deleteLoopDesignMock(code: string) {
  if (isLoopUsedByAgentMock(code))
    throw new LoopDesignMockError("inUse");
  if (!designs.some((design) => design.loopCode === code))
    throw new LoopDesignMockError("notFound");
  designs = designs.filter((d) => d.loopCode !== code);
}
