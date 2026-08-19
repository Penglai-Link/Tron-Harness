// @vitest-environment jsdom

import { describe, expect, it } from "vitest";

import {
  createMockQuickResource,
  deleteMockResource,
  getWorkspaceResourceDetail,
  replaceMockResourceFile,
  updateMockResource,
} from "./data";

describe("resource mock state", () => {
  it("keeps quick-upload records available to detail, update, replace, and delete flows", () => {
    const workspaceCode = "MOCK_LIFECYCLE";
    const resourceCode = "mock-lifecycle-file";
    createMockQuickResource({
      allowInheritance: false,
      description: "before",
      file: new File(["first"], "first.txt", { type: "text/plain" }),
      kind: "FILE",
      name: "Before",
      pageCount: null,
      resourceCode,
      workspaceCode,
    });

    expect(getWorkspaceResourceDetail(workspaceCode, "", resourceCode)?.name).toBe("Before");
    expect(updateMockResource(workspaceCode, resourceCode, {
      allowInheritance: true,
      attributes: null,
      description: "after",
      name: "After",
    })?.name).toBe("After");
    expect(replaceMockResourceFile(
      workspaceCode,
      resourceCode,
      new File(["second"], "second.pdf", { type: "application/pdf" }),
    )?.file?.fileName).toBe("second.pdf");

    expect(deleteMockResource(workspaceCode, resourceCode)).toBe(true);
    expect(getWorkspaceResourceDetail(workspaceCode, "", resourceCode)).toBeNull();
  });
});
