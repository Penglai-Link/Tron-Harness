import { describe, expect, it } from "vitest";
import { emptyProviderDraft, presetModelToDraft, providerDraftToPayload, validateProviderDraft } from "./configuredProviderForm";

function validDraft() {
  const draft = emptyProviderDraft();
  draft.providerId = "local-gateway";
  draft.displayName = "Local gateway";
  draft.baseUrl = "http://localhost:8080/v1";
  draft.models[0]!.modelId = "model";
  draft.models[0]!.name = "Model";
  return draft;
}

describe("configured provider constraints", () => {
  it("allows a local service without a key, preserving unknown limits as null", () => {
    const draft = validDraft();
    expect(validateProviderDraft(draft)).toBeNull();
    expect(providerDraftToPayload(draft)).toMatchObject({ apiKey: null, models: [{ contextWindowTokens: null, maxOutputTokens: null }] });
  });

  it.each(["authorization", "X-Api-Key", "Host", "Content-Length", "Cookie"])("rejects reserved header %s", (name) => {
    const draft = validDraft();
    draft.headers = [{ rowId: "one", name, value: "secret" }];
    expect(validateProviderDraft(draft)).toBe("headers");
  });

  it("rejects duplicate headers case-insensitively and header injection", () => {
    const draft = validDraft();
    draft.headers = [{ rowId: "one", name: "X-Client", value: "Tron" }, { rowId: "two", name: "x-client", value: "another" }];
    expect(validateProviderDraft(draft)).toBe("headers");
    draft.headers = [{ rowId: "one", name: "x-client", value: "Tron\r\nAuthorization: secret" }];
    expect(validateProviderDraft(draft)).toBe("headers");
  });

  it("does not advertise audio or video unsupported by the Tron conversation bridge", () => {
    const candidate = { modelId: "unknown", name: "Unknown", contextWindowTokens: 256000, reasoning: true, inputCapabilities: ["text", "image", "audio", "video"] as const, toolUse: true };
    const openai = presetModelToDraft({ ...candidate, inputCapabilities: [...candidate.inputCapabilities] }, "OpenAiChatCompletions");
    const gemini = presetModelToDraft({ ...candidate, inputCapabilities: [...candidate.inputCapabilities] }, "GoogleGemini");
    expect(openai.inputCapabilities).toEqual(["Text", "Image"]);
    expect(openai.maxOutputTokens).toBe("");
    expect(gemini.inputCapabilities).toEqual(["Text", "Image"]);
  });
});
