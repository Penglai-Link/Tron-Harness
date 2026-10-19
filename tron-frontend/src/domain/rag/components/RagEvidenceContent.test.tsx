// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { RagEvidence } from "../model/RagModel";
import { pages } from "@/i18n/locales/en-US/pages";
import { RagEvidenceContent } from "./RagEvidenceContent";

const preview = vi.hoisted(() => vi.fn());
vi.mock("@/domain/knowledge-base/api/knowledgeBaseApi", () => ({ downloadKnowledgeResource: vi.fn() }));
vi.mock("@/domain/resource/components/ResourceFilePreview", () => ({ ResourceFilePreview: (props: { fileName: string }) => { preview(props); return <img alt={props.fileName} />; } }));
vi.mock("./RagEvidenceText", () => ({ RagEvidenceText: ({ text }: { text: string }) => <p>{text}</p> }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, args?: Record<string, unknown>) => String(key.split(".").reduce((value: unknown, part) => (value as Record<string, unknown>)?.[part], pages) ?? key).replace(/{{(\w+)}}/g, (_, part) => String(args?.[part] ?? "")), i18n: { language: "en-US" } }) }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
const hit = (): RagEvidence => ({ corpusId: "kb", sourceCode: "source", sourceVersion: "publication", chunkCode: "body", title: "Manual", text: "combined compatibility text", source: { documentId: "doc" }, location: { page: 4 }, score: 1, channels: ["vector"],
  parts: [{ chunkCode: "body", text: "Relevant source paragraph", imageVector: false, fileName: "manual.pdf", location: { page: 4 }, sourceLocations: [{ page: 4 }] }, { chunkCode: "pixels", text: "Image index input", imageVector: true, fileName: "manual.pdf", location: {}, sourceLocations: [] }],
  images: [{ id: "figure", resourceCode: "asset", workspaceCode: "PLATFORM", fileName: "manual.pdf", location: { page: 4 }, containerLocation: null, annotations: [{ kind: "SourceCaption", text: "Figure 1 rating", source: "parser", chunkCodes: ["caption"] }, { kind: "GeneratedDescription", text: "Description of the rating diagram", source: "vision", chunkCodes: ["caption"] }] }],
});
it("shows grouped body, original image, attributed description and file position", () => {
  render(<RagEvidenceContent hit={hit()} libraryId="kb" />);
  expect(screen.getByText("Relevant source paragraph")).toBeTruthy();
  expect(screen.getByText("Figure 1 rating")).toBeTruthy();
  expect(screen.getByText("Generated image description")).toBeTruthy();
  expect(screen.getByText("Description of the rating diagram")).toBeTruthy();
  expect(screen.getByRole("img")).toBeTruthy();
  expect(screen.getAllByText(/manual.pdf.*4/).length).toBeGreaterThan(0);
  expect(screen.queryByText("Image index input")).toBeNull();
  expect(preview.mock.calls[0][0]).toMatchObject({ resourceCode: "asset", cacheScope: "knowledge-library:kb", format: "image" });
  expect(typeof preview.mock.calls[0][0].loadBinary).toBe("function");
});
it("preserves text-only results and does not invent a page for an unmapped image", () => {
  const row = hit(); row.parts = undefined; row.images = undefined;
  const view = render(<RagEvidenceContent hit={row} />);
  expect(screen.getByText("combined compatibility text")).toBeTruthy();
  expect(screen.queryByRole("img")).toBeNull();
  const image = hit(); image.images![0].location = { sourceSegment: 1, segmentPage: 3 };
  view.rerender(<RagEvidenceContent hit={image} libraryId="kb" />);
  expect(screen.getByText(/manual.pdf.*Segment 2.*3/)).toBeTruthy();
});
