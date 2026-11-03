// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import { pages } from "@/i18n/locales/en-US/pages";
import type { RagGraphWindow } from "../model/RagGraphModel";
import { RagGraphPanel } from "./RagGraphPanel";
const api = vi.hoisted(() => ({ browse: vi.fn(), search: vi.fn(), expand: vi.fn(), corpora: vi.fn() }));
vi.mock("../api/ragGraphApi", () => ({ browseRagGraph: api.browse, searchRagGraph: api.search, expandRagGraph: api.expand }));
vi.mock("@/domain/rag/api/ragApi", () => ({ listRagCorpora: api.corpora }));
vi.mock("@/shared", () => ({ SurfaceCard: ({ children }: { children: React.ReactNode }) => <div>{children}</div> }));
vi.mock("./GraphCanvas", () => ({ GraphCanvas: ({ nodes, onSelectNode }: { nodes: { nodeCode: string; properties: { name: string } }[]; onSelectNode: (id: string) => void }) => <div aria-label="Graph canvas">{nodes.map((node) => <button key={node.nodeCode} onClick={() => onSelectNode(node.nodeCode)}>{node.properties.name}</button>)}</div> }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, args?: Record<string, unknown>) => String(key.split(".").reduce((v: unknown, k) => (v as Record<string, unknown>)?.[k], pages) ?? args?.defaultValue ?? key).replace(/{{(\w+)}}/g, (_, k) => String(args?.[k] ?? "")), i18n: { resolvedLanguage: "en-US" } }) }));
const fixture = (): RagGraphWindow => ({ snapshotId: "snapshot", seeds: [{ id: "alpha", score: 1, channels: ["fulltext"] }],
  nodes: [{ id: "alpha", corpusId: "kb", kind: "Entity", name: "Alpha", entityType: "Service", descriptions: [], aliases: [], referenceIds: ["r1"], hops: [{ seedId: "alpha", nodeId: "alpha", direction: "Outgoing", hops: 0 }] },
    { id: "beta", corpusId: "kb", kind: "Entity", name: "Beta", entityType: "Service", descriptions: [], aliases: [], referenceIds: ["r1"], hops: [] }],
  edges: [{ id: "calls", from: "alpha", to: "beta", relationship: "CALLS", structural: false, referenceIds: ["r1"] }],
  references: [{ id: "r1", corpusId: "kb", sourceCode: "src", sourceVersion: "v1", generationId: "g", chunkCode: "c", fileName: "manual.md", documentId: null, resourceCode: null, workspaceCode: "PLATFORM", quote: "Alpha calls Beta.", location: { page: 4 } }],
  stages: [], nextCursor: "cursor", expiresAt: 100, unfinished: [{ nodeId: "alpha", direction: "Outgoing", depth: 1, phase: "Traversal" }], truncatedBy: ["EdgeBudget"], durationMs: 5 });
beforeEach(() => { vi.clearAllMocks(); api.corpora.mockResolvedValue([]); api.browse.mockResolvedValue(fixture()); api.search.mockResolvedValue(fixture()); api.expand.mockResolvedValue({ ...fixture(), nextCursor: null, unfinished: [], truncatedBy: [] }); });
afterEach(cleanup);
function mount() { render(<MemoryRouter><RagGraphPanel /></MemoryRouter>); }
it("searches on submit, shows original references, and continues with cursor alone", async () => {
  mount(); await waitFor(() => expect(api.browse).toHaveBeenCalledTimes(1)); await screen.findByRole("button", { name: "Alpha" }); fireEvent.change(screen.getByLabelText("Question"), { target: { value: "Alpha" } }); expect(api.search).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  await screen.findByRole("button", { name: "Alpha" });
  expect(api.search.mock.calls[0][0]).toMatchObject({ query: "Alpha", corpusIds: [], incomingDepth: 3, outgoingDepth: 3 });
  fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
  const sources = within(screen.getByRole("region", { name: "Original sources" }));
  expect(await sources.findByText("manual.md")).toBeTruthy(); expect(sources.getByText("Alpha calls Beta.")).toBeTruthy(); expect(screen.getByText("Page: 4")).toBeTruthy();
  const related = screen.getByRole("button", { name: "Inspect related entity: Beta" });
  expect(related.title).toBe("Alpha → CALLS → Beta");
  expect(screen.getAllByText("Alpha calls Beta.")).toHaveLength(1);
  fireEvent.click(screen.getByRole("button", { name: "Load more" }));
  await waitFor(() => expect(api.expand).toHaveBeenCalledTimes(1)); expect(api.expand.mock.calls[0][0]).toEqual({ cursor: "cursor" });
  await waitFor(() => expect(screen.queryByRole("button", { name: "Load more" })).toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "Expand incoming" }));
  await waitFor(() => expect(api.expand).toHaveBeenCalledTimes(2));
  expect(api.expand.mock.calls[1][0]).toMatchObject({ nodeIds: ["alpha"], incomingDepth: 3, outgoingDepth: 0 });
  expect(api.search).toHaveBeenCalledTimes(1);
});
it("clears graph and inspector evidence after continuation authorization fails", async () => {
  mount(); await waitFor(() => expect(api.browse).toHaveBeenCalledTimes(1)); await screen.findByRole("button", { name: "Alpha" }); fireEvent.change(screen.getByLabelText("Question"), { target: { value: "Alpha" } }); fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Alpha" })); await within(screen.getByRole("region", { name: "Original sources" })).findByText("manual.md");
  api.expand.mockRejectedValue(new Error("Authorization changed; search again")); fireEvent.click(screen.getByRole("button", { name: "Load more" }));
  expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Authorization changed; search again");
  expect(screen.queryByText("manual.md")).toBeNull(); expect(screen.queryByRole("button", { name: "Alpha" })).toBeNull();
});

it("loads published graphs before a question is entered and can refresh the overview", async () => {
  mount();
  await screen.findByRole("button", { name: "Alpha" });
  expect(api.browse).toHaveBeenCalledTimes(1);
  expect(api.browse.mock.calls[0][0]).toMatchObject({ corpusIds: [], seedCount: 50, nodeLimit: 100, edgeLimit: 200 });
  expect(api.search).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Browse graph" }));
  await waitFor(() => expect(api.browse).toHaveBeenCalledTimes(2));
});

it("shows useful details once and omits traversal diagnostics and storage identifiers", async () => {
  const data = fixture();
  data.nodes[0].descriptions = [
    { text: "Alpha", referenceId: "r1" },
    { text: "A useful service description.", referenceId: "r1" },
    { text: "A useful service description.", referenceId: "r2" },
    { text: " ", referenceId: "r2" },
  ];
  data.nodes[0].aliases = [
    { text: "Alpha", referenceId: "r1" },
    { text: "First service", referenceId: "r1" },
    { text: "First service", referenceId: "r2" },
  ];
  data.nodes[0].hops = Array.from({ length: 20 }, (_, index) => ({ seedId: `debug-seed-${index}`, nodeId: "alpha", direction: "Incoming", hops: 1 }));
  data.nodes[0].referenceIds.push("r2", "r3");
  data.references.push({ ...data.references[0], id: "r2", chunkCode: "debug-chunk-id" });
  data.references.push({ ...data.references[0], id: "r3", quote: "Another useful source passage.", location: { page: 5 } });
  api.browse.mockResolvedValue(data);
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Alpha" }));
  expect(screen.getByRole("heading", { name: "Alpha" })).toBeTruthy();
  expect(screen.getAllByText("A useful service description.")).toHaveLength(1);
  expect(screen.getByText("Aliases: First service")).toBeTruthy();
  expect(screen.queryByText("Hit reasons")).toBeNull();
  expect(screen.queryByText(/hops along/)).toBeNull();
  expect(screen.queryByText(/debug-seed|debug-chunk-id/)).toBeNull();
  const sources = within(screen.getByRole("region", { name: "Original sources" }));
  expect(sources.getAllByText("manual.md")).toHaveLength(2);
  expect(sources.getAllByText("Alpha calls Beta.")).toHaveLength(1);
  expect(sources.getByText("Another useful source passage.")).toBeTruthy();
  expect(sources.getByText("Page: 5")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Inspect related entity: Beta" }));
  expect(screen.getByRole("heading", { name: "Beta" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Close node inspector" }));
  expect(screen.queryByRole("heading", { name: "Beta" })).toBeNull();
});
