const variableNamePattern = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;

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

function walkPromptTemplate(
  promptBody: string,
  replace: (variableName: string, placeholder: string) => string,
) {
  const variables: string[] = [];
  const invalidVariables: string[] = [];
  const seenVariables = new Set<string>();
  const seenInvalidVariables = new Set<string>();
  let renderedPrompt = "";
  let cursor = 0;

  while (cursor < promptBody.length) {
    if (promptBody.startsWith(String.raw`\{{`, cursor)) {
      renderedPrompt += "{{";
      cursor += 3;
      continue;
    }
    if (promptBody.startsWith("{{", cursor)) {
      const contentStart = cursor + 2;
      const contentEnd = promptBody.indexOf("}}", contentStart);
      if (contentEnd < 0) {
        pushOnce(invalidVariables, seenInvalidVariables, promptBody.slice(contentStart) || "unclosed placeholder");
        renderedPrompt += promptBody.slice(cursor);
        break;
      }
      const variableName = promptBody.slice(contentStart, contentEnd);
      const placeholder = promptBody.slice(cursor, contentEnd + 2);
      if (variableNamePattern.test(variableName)) {
        pushOnce(variables, seenVariables, variableName);
        renderedPrompt += replace(variableName, placeholder);
      } else {
        pushOnce(invalidVariables, seenInvalidVariables, variableName);
        renderedPrompt += placeholder;
      }
      cursor = contentEnd + 2;
      continue;
    }
    const codePoint = promptBody.codePointAt(cursor);
    if (codePoint === undefined) break;
    const character = String.fromCodePoint(codePoint);
    renderedPrompt += character;
    cursor += character.length;
  }

  return { invalidVariables, renderedPrompt, variables };
}

export function analyzePromptTemplate(promptBody: string): PromptTemplateAnalysis {
  const { invalidVariables, variables } = walkPromptTemplate(promptBody, (_name, placeholder) => placeholder);
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

  const { renderedPrompt } = walkPromptTemplate(promptBody, (variableName, placeholder) => (
    Object.hasOwn(variableValues, variableName) ? variableValues[variableName] : placeholder
  ));

  return { errors, renderedPrompt, variables: analysis.variables };
}
