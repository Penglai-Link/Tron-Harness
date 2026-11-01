import { describe, expect, it } from "vitest";
import { duplicateProviderDraft, emptyProviderDraft, presetModelToDraft, presetToDraft, providerDraftToPayload, validateProviderDraft } from "./configuredProviderForm";

function validDraft() {
  const draft = emptyProviderDraft();
  draft.displayName = "Local gateway";
  draft.baseUrl = "http://localhost:8080/v1";
  draft.models[0]!.modelId = "model";
  draft.models[0]!.name = "Model";
  return draft;
}

describe("configured provider constraints", () => {
  it("leaves connection identity to the backend for provider presets", () => {
    const preset = { id: "sample", name: "Sample", protocol: "OpenAiChatCompletions" as const, baseUrl: "https://sample.test/v1", envKey: null, guessed: false, needsBaseUrl: false, supported: true, unsupportedReason: null, modelCount: 0 };
    const first = presetToDraft(preset)!;
    const second = presetToDraft(preset)!;
    expect(first).not.toHaveProperty("providerId");
    expect(providerDraftToPayload(second)).not.toHaveProperty("providerId");
    expect(first.baseUrl).toBe(second.baseUrl);
    expect(first.displayName).toBe("Sample");
    expect(second.apiKey).toBe("");
  });

  it("copies connection settings with fresh identities and no inherited credentials", () => {
    const original = { ...providerDraftToPayload(validDraft()), providerId: "original-connection", hasApiKey: true, apiKeySummary: "sk-123***sdf-hsd", createdAt: 1, updatedAt: 2 };
    original.models[0]!.modelCode = "original-model-code";
    const copy = duplicateProviderDraft(original);
    expect(copy).not.toHaveProperty("providerId");
    expect(providerDraftToPayload(copy)).not.toHaveProperty("providerId");
    expect(copy.models[0]!.modelCode).toBeNull();
    expect(copy.models[0]!.modelId).toBe(original.models[0]!.modelId);
    expect(copy.apiKey).toBe("");
    expect(copy.hasApiKey).toBe(false);
    expect(copy.apiKeySummary).toBeNull();
    expect(copy.baseUrl).toBe(original.baseUrl);
  });
  it.each(["Image", "Video", "SpeechSynthesis", "SpeechRecognition", "RealtimeSpeech"] as const)("preserves %s classification and media inputs in the saved payload", (modelType) => {
    const draft = validDraft();
    draft.models[0]!.modelType = modelType;
    draft.models[0]!.inputCapabilities = modelType === "SpeechRecognition" ? ["Audio"] : ["Text", "Image", "Audio", "Video"];
    expect(validateProviderDraft(draft)).toBeNull();
    expect(providerDraftToPayload(draft).models[0]).toMatchObject({ modelType, inputCapabilities: draft.models[0]!.inputCapabilities });
    draft.models[0]!.inputCapabilities = [];
    expect(validateProviderDraft(draft)).toBe("inputCapabilities");
  });
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


it("distinguishes text and multimodal embedding and rerank input contracts", () => {
  for (const type of ["TextEmbedding", "TextRerank"] as const) {
    const draft = { ...emptyProviderDraft(), displayName: "Provider", baseUrl: "https://provider.test/v1" };
    draft.models[0] = { ...draft.models[0], modelType: type, dimension: "1024", modelId: "model", name: "Model", inputCapabilities: ["Text"] };
    expect(validateProviderDraft(draft)).toBeNull();
    draft.models[0].inputCapabilities.push("Image");
    expect(validateProviderDraft(draft)).toBe("inputCapabilities");
  }
  for (const type of ["MultimodalEmbedding", "MultimodalRerank"] as const) {
    const draft = { ...emptyProviderDraft(), displayName: "Provider", baseUrl: "https://provider.test/v1" };
    draft.models[0] = { ...draft.models[0], modelType: type, dimension: "1024", modelId: "model", name: "Model", inputCapabilities: ["Text", "Image"] };
    expect(validateProviderDraft(draft)).toBeNull();
    draft.models[0].inputCapabilities = ["Text"];
    expect(validateProviderDraft(draft)).toBe("inputCapabilities");
  }
});

it("requires and serializes embedding dimensions", () => {
  const draft = { ...emptyProviderDraft(), displayName: "Provider", baseUrl: "https://provider.test/v1" };
  draft.models[0] = { ...draft.models[0], modelType: "TextEmbedding", modelId: "embed", name: "Embed" };
  expect(validateProviderDraft(draft)).toBe("dimension");
  draft.models[0].dimension = "1024";
  expect(validateProviderDraft(draft)).toBeNull();
  expect(providerDraftToPayload(draft).models[0].dimension).toBe(1024);
});
