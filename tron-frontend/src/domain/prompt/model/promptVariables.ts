import type { PromptVariable } from "./PromptModel";
import { analyzePromptTemplate } from "./promptTemplate";

export const promptVariableDescriptionMaxLength = 200;

export function managedPromptVariable(name: string): PromptVariable {
  return {
    description: "",
    name,
    type: "STRING",
    sourceKind: "MANAGED",
    managedVariableKey: null,
    runtimeProvider: null,
  };
}

export function reconcilePromptVariables(
  promptBody: string,
  currentVariables: PromptVariable[],
): PromptVariable[] {
  const currentByName = new Map(currentVariables.map((variable) => [variable.name, variable]));
  return analyzePromptTemplate(promptBody).variables.map((name) => {
    const current = currentByName.get(name);
    return current ?? managedPromptVariable(name);
  });
}

export function promptVariablesAreReady(promptBody: string, variables: PromptVariable[]) {
  const analysis = analyzePromptTemplate(promptBody);
  if (analysis.invalidVariables.length || analysis.variables.length !== variables.length) return false;
  const expectedNames = new Set(analysis.variables);
  const actualNames = new Set(variables.map((variable) => variable.name));
  if (actualNames.size !== variables.length || expectedNames.size !== actualNames.size) return false;
  return variables.every((variable) => {
    const descriptionLength = Array.from(variable.description.trim()).length;
    const bindingReady = variable.sourceKind === "MANAGED"
      ? Boolean(variable.managedVariableKey?.trim()) && !variable.runtimeProvider
      : variable.sourceKind === "RUNTIME"
        ? Boolean(variable.runtimeProvider) && !variable.managedVariableKey
        : false;
    return expectedNames.has(variable.name)
      && descriptionLength >= 1
      && descriptionLength <= promptVariableDescriptionMaxLength
      && bindingReady;
  });
}

export function normalizePromptVariables(promptBody: string, variables: PromptVariable[]) {
  if (!promptVariablesAreReady(promptBody, variables)) {
    throw new Error("Prompt variables must match the template and include descriptions");
  }
  const byName = new Map(variables.map((variable) => [variable.name, variable]));
  return analyzePromptTemplate(promptBody).variables.map((name) => {
    const variable = byName.get(name)!;
    return {
      ...variable,
      description: variable.description.trim(),
      managedVariableKey: variable.sourceKind === "MANAGED"
        ? variable.managedVariableKey!.trim()
        : null,
      runtimeProvider: variable.sourceKind === "RUNTIME"
        ? variable.runtimeProvider!
        : null,
    };
  });
}
