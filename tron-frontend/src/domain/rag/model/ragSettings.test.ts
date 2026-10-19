import { describe, expect, it } from "vitest";
import { canSaveRagSettings, defaultRagSettings } from "./ragSettings";

describe("knowledge graph index requirements", () => {
  it("requires embedding and extraction models for graph indexes even when chunk vectors are off", () => {
    const settings = { ...defaultRagSettings(), vector: false };
    expect(settings.graph).toBe(true);
    expect(canSaveRagSettings(settings)).toBe(true);
    expect(canSaveRagSettings({ ...settings, embeddingModelCode: null })).toBe(false);
    expect(canSaveRagSettings({ ...settings, graphModelCode: null })).toBe(false);
    expect(canSaveRagSettings({ ...settings, graph: false, embeddingModelCode: null, graphModelCode: null })).toBe(true);
  });
});
