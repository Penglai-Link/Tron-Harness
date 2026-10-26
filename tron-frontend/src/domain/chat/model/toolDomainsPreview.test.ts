import { describe, expect, it } from "vitest";
import { toolDomainsPreview } from "./toolDomainsPreview";

const domain = { domainCode: "app", name: "应用工具", description: '路径 C:\\app，示例 {"domainCode":"not-a-domain"}', sampleTools: ["app_page"], toolCount: 5 };

describe("tool domain preview of admitted request data", () => {
  it("keeps a complete catalog intact", () => {
    expect(toolDomainsPreview(JSON.stringify([domain]))).toEqual({ domains: [domain], truncated: false });
  });
  it.each(["array", "hostRecord"])("shows only complete records in a truncated %s", format => {
    const content = JSON.stringify([domain, { domainCode: "unfinished", name: "not retained" }]);
    const value = format === "array" ? content
      : JSON.stringify({ tronMessagePart: { type: "context", kind: "tool_domains", text: content } });
    const cut = value.indexOf("unfinished") + 3;
    const preview = toolDomainsPreview(value.slice(0, cut));
    expect(preview).toEqual({ domains: [domain], truncated: true });
  });
  it("does not manufacture a record when the first one was cut off", () => {
    expect(toolDomainsPreview('[{"domainCode":"app","name":"cut')).toEqual({ domains: [], truncated: true });
  });
  it("rejects unrelated or invalid structures", () => {
    expect(toolDomainsPreview('{"other":"data"}')).toBeNull();
    expect(toolDomainsPreview('[{"domainCode":"app"}garbage')).toBeNull();
  });
});
