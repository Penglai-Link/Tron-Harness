import { createInstance } from "i18next";
import { describe, expect, it } from "vitest";
import { pages as en } from "@/i18n/locales/en-US/pages";
import { pages as zh } from "@/i18n/locales/zh-CN/pages";
import { formatKnowledgeDocumentError } from "./knowledgeDocumentError";

async function translator(locale: "en-US" | "zh-CN") {
  const i18n = createInstance();
  await i18n.init({ lng: locale, resources: { "en-US": { pages: en }, "zh-CN": { pages: zh } }, interpolation: { escapeValue: false } });
  return i18n.getFixedT(locale, "pages");
}

describe.each(["en-US", "zh-CN"] as const)("MinerU errors in %s", (locale) => {
  it("localizes the failure and keeps the actual model diagnostic and job", async () => {
    const t = await translator(locale);
    const reason = "[ONNXRuntimeError] INVALID_PROTOBUF: model failed to load";
    const error = `Invalid request: MINERU_PARSE_FAILED:${JSON.stringify({ jobId: "job-real", reason })}`;
    const result = formatKnowledgeDocumentError(error, t);
    expect(result).toContain(reason);
    expect(result).toContain("job-real");
    expect(result).toContain(locale === "zh-CN" ? "MinerU 解析失败" : "MinerU parsing failed");
    expect(result).not.toContain("MINERU_PARSE_FAILED:");
  });
  it("explains when the service supplied no diagnostic", async () => {
    const t = await translator(locale);
    const error = `MINERU_PARSE_FAILED:${JSON.stringify({ jobId: "job-1", reason: null })}`;
    expect(formatKnowledgeDocumentError(error, t)).toContain(t("knowledgeLibrary.parseFailureUnknown"));
  });
  it("preserves unknown and malformed errors", async () => {
    const t = await translator(locale);
    for (const error of ["Embedding unavailable", "MINERU_PARSE_FAILED:broken", "MINERU_PARSE_FAILED:{}"]) {
      expect(formatKnowledgeDocumentError(error, t)).toBe(error);
    }
    expect(formatKnowledgeDocumentError(null, t)).toBe("");
  });
});
