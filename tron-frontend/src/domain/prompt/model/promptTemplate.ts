const variableNamePattern = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;
const placeholderPattern = /(\\)?\{\{([^{}]*)\}\}/g;

export interface PromptTemplateAnalysis {
  variables: string[];
  invalidVariables: string[];
}

export interface PromptTemplateError {
  kind: "INVALID" | "MISSING" | "UNKNOWN";
  variableName: string;
}

function pushOnce(values: string[], seen: Set<string>, value: string) {
  if (seen.has(value)) return;
  seen.add(value);
  values.push(value);
}

export function analyzePromptTemplate(promptBody: string): PromptTemplateAnalysis {
  const variables: string[] = [];
  const invalidVariables: string[] = [];
  const seenVariables = new Set<string>();
  const seenInvalidVariables = new Set<string>();

  for (const match of promptBody.matchAll(placeholderPattern)) {
    if (match[1]) continue;
    const variableName = match[2];
    if (variableNamePattern.test(variableName)) {
      pushOnce(variables, seenVariables, variableName);
    } else {
      pushOnce(invalidVariables, seenInvalidVariables, variableName);
    }
  }

  return { invalidVariables, variables };
}

export function renderPromptTemplate(
  promptBody: string,
  variableValues: Record<string, string>,
) {
  const analysis = analyzePromptTemplate(promptBody);
  const knownVariables = new Set(analysis.variables);
  const errors: PromptTemplateError[] = [
    ...analysis.invalidVariables.map((variableName) => ({
      kind: "INVALID" as const,
      variableName,
    })),
    ...analysis.variables
      .filter((variableName) => !Object.hasOwn(variableValues, variableName))
      .map((variableName) => ({ kind: "MISSING" as const, variableName })),
    ...Object.keys(variableValues)
      .filter((variableName) => !knownVariables.has(variableName))
      .map((variableName) => ({ kind: "UNKNOWN" as const, variableName })),
  ];

  const renderedPrompt = promptBody.replace(
    placeholderPattern,
    (placeholder, escaped: string | undefined, variableName: string) => {
      if (escaped) return placeholder.slice(1);
      if (!variableNamePattern.test(variableName)) return placeholder;
      return Object.hasOwn(variableValues, variableName)
        ? variableValues[variableName]
        : placeholder;
    },
  );

  return { errors, renderedPrompt, variables: analysis.variables };
}
