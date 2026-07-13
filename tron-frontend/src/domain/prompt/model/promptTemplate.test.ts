import { describe, expect, it } from "vitest";

import {
  analyzePromptTemplate,
  renderPromptTemplate,
} from "./promptTemplate";

describe("prompt template variables", () => {
  it("detects valid variables once and preserves first-seen order", () => {
    expect(analyzePromptTemplate(
      "Hello {{user_name}}, use {{_locale}} then {{user_name}} and {{a1}}.",
    )).toEqual({
      invalidVariables: [],
      variables: ["user_name", "_locale", "a1"],
    });
  });

  it("reports invalid names and ignores escaped placeholders", () => {
    expect(analyzePromptTemplate(
      String.raw`Keep \{{literal_name}}, reject {{9name}}, {{has-dash}}, and {{${"a".repeat(65)}}}.`,
    )).toEqual({
      invalidVariables: ["9name", "has-dash", "a".repeat(65)],
      variables: [],
    });
  });

  it("replaces every occurrence without recursively expanding replacement values", () => {
    expect(renderPromptTemplate(
      "{{greeting}}, {{name}}. Again: {{name}}.",
      { greeting: "Hello {{name}}", name: "Ada" },
    )).toEqual({
      errors: [],
      renderedPrompt: "Hello {{name}}, Ada. Again: Ada.",
      variables: ["greeting", "name"],
    });
  });

  it("unescapes literal placeholders and reports missing and unknown values", () => {
    expect(renderPromptTemplate(
      String.raw`Use \{{literal}} with {{known}} and {{missing}}.`,
      { known: "value", extra: "unused" },
    )).toEqual({
      errors: [
        { kind: "MISSING", variableName: "missing" },
        { kind: "UNKNOWN", variableName: "extra" },
      ],
      renderedPrompt: "Use {{literal}} with value and {{missing}}.",
      variables: ["known", "missing"],
    });
  });
});
