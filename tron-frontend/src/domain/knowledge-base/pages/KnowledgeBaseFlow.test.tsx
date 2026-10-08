// @vitest-environment jsdom
import type { ReactNode, InputHTMLAttributes, SelectHTMLAttributes } from "react";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router";
import { pages } from "@/i18n/locales/en-US/pages";
import type { ParsingTier } from "../model/KnowledgeLibraryModel";
const fixture = vi.hoisted(() => ({ workspace: "space-a", user: "user-a", warnings: [] as string[], filename: "policy.md", status: "FAILED", stage: undefined as string | null | undefined, hasArtifact: true, parsingTier: null as ParsingTier | null, fidelityIssues: [] as {kind: string; page: number; blocks: string[]; sourceExcerpt: string; imageResourceCodes: string[]}[], cancel: vi.fn(), reparse: vi.fn(), capabilities: vi.fn(), create: vi.fn(), update: vi.fn(), preview: vi.fn(), index: vi.fn(), grants: vi.fn(), upload: vi.fn(), error: vi.fn(), success: vi.fn() }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, args?: Record<string, unknown>) => { const pluralKey = args?.count !== undefined ? `${key}_${args.count === 1 ? "one" : "other"}` : key; const pluralValue = pluralKey.split(".").reduce((node: unknown, item) => (node as Record<string, unknown>)?.[item], pages); const value = pluralValue ?? key.split(".").reduce((node: unknown, item) => (node as Record<string, unknown>)?.[item], pages); return String(value ?? args?.defaultValue ?? key).replace(/{{(\w+)}}/g, (_, k) => String(args?.[k] ?? "")); }, i18n: { resolvedLanguage: "en-US" } }) }));
vi.mock("@/shared/hooks/useLocaleFormatters", () => ({ useLocaleFormatters: () => ({ formatNumber: (value: number) => String(value) }) }));
vi.mock("@/store/authStore", () => ({ useAuthStore: (selector: (state: unknown) => unknown) => selector({ session: { user: { userCode: fixture.user } } }) }));
vi.mock("@/shared", () => ({ useWorkspaceSpaces: () => ({ selectedSecondarySpace: { workspaceCode: fixture.workspace } }), PageHeader: ({ title, actions }: { title: string; actions: ReactNode }) => <header><h1>{title}</h1>{actions}</header>, SurfaceCard: ({ children }: { children: ReactNode }) => <section>{children}</section> }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ success: fixture.success, error: fixture.error }) }));
vi.mock("@/components/ui/dropdown-menu", () => ({
 DropdownMenu: ({ children }: { children: ReactNode }) => <div>{children}</div>,
 DropdownMenuTrigger: ({ render }: { render: ReactNode }) => render,
 DropdownMenuContent: ({ children }: { children: ReactNode }) => <div role="menu">{children}</div>,
 DropdownMenuItem: ({ children, disabled, onClick }: { children: ReactNode; disabled?: boolean; onClick: () => void }) => <button role="menuitem" aria-disabled={disabled || undefined} disabled={disabled} onClick={onClick}>{children}</button>,
}));
vi.mock("@/components/ui/dialog", () => ({ Dialog: ({ children, open }: { children: ReactNode; open: boolean }) => open ? <div role="dialog">{children}</div> : null, DialogContent: ({ children }: { children: ReactNode }) => <div>{children}</div>, DialogHeader: ({ children }: { children: ReactNode }) => <div>{children}</div>, DialogFooter: ({ children }: { children: ReactNode }) => <div>{children}</div>, DialogTitle: ({ children }: { children: ReactNode }) => <h2>{children}</h2>, DialogDescription: ({ children }: { children: ReactNode }) => <p>{children}</p> }));
vi.mock("@/components/forms", () => ({ Field: ({ label, children }: { label: string; children: ReactNode }) => <label>{label}{children}</label>, TextInput: (props: InputHTMLAttributes<HTMLInputElement>) => <input {...props} />, TextArea: (props: React.ComponentProps<"textarea">) => <textarea {...props} />, SelectInput: ({ onValueChange, ...props }: SelectHTMLAttributes<HTMLSelectElement> & { onValueChange: (value: string) => void }) => <select {...props} onChange={(event) => onValueChange(event.target.value)} /> }));
vi.mock("@/components/ui/select", () => ({ SelectItem: ({ value, children }: { value: string; children: ReactNode }) => <option value={value}>{children}</option> }));
vi.mock("@/components/ui/switch", () => ({ Switch: ({ checked, disabled, onCheckedChange, ...props }: { checked: boolean; disabled?: boolean; onCheckedChange: (checked: boolean) => void }) => <input {...props} type="checkbox" role="switch" checked={checked} disabled={disabled} onChange={(event) => onCheckedChange(event.target.checked)} /> }));
vi.mock("@/domain/resource/api/resourceApi", () => ({ downloadResourceBinary: vi.fn() }));
vi.mock("@/domain/agent/api/agentManagementApi", () => ({ fetchWorkspaceInternalAgents: vi.fn(async () => [{ agentCode: "main-agent", name: "Main Agent" }]) }));
vi.mock("../api/knowledgeBaseApi", () => ({
 reparseVisualKnowledgeDocument: fixture.reparse, getParsingCapabilities: fixture.capabilities,
 listKnowledgeLibraries: vi.fn(async () => []), createKnowledgeLibrary: fixture.create,
 getKnowledgeLibrary: vi.fn(async () => ({ id: "kb-a", name: "Policies", description: "", workspaceCode: "space-a", documentCount: 1, agentCodes: [], defaultChunkConfig: { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 }, documents: [{ id: "doc-a", name: "Policy", fileName: fixture.user === "user-a" ? fixture.filename : "other-policy.md", resourceCode: "res-a", resourceRevision: 1, chunkConfig: null, status: fixture.status, stage: fixture.stage !== undefined ? fixture.stage : ["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(fixture.status) ? ({ QUEUED: "Parsing", PARSING: "Parsing", CHUNKING: "Chunking", EMBEDDING: "Embedding", PUBLISHING: "Publishing" } as Record<string,string>)[fixture.status] : null, error: fixture.status === "REVIEW_REQUIRED" ? "Source review required" : "Embedding unavailable", parsingTier: fixture.parsingTier, parsingArtifact: fixture.hasArtifact ? {effectiveTier: "standard", parserVersion: "4.0.10", resourceCode: "cache-a", imageResourceCodes: [], fidelityIssues: fixture.fidelityIssues} : null, warnings: fixture.warnings, chunkCount: 0, indexRevision: 0 }] })),
 cancelKnowledgeDocument: fixture.cancel, updateKnowledgeDocument: fixture.update, previewKnowledgeDocument: fixture.preview, indexKnowledgeDocument: fixture.index, setKnowledgeLibraryGrants: fixture.grants, uploadKnowledgeDocument: fixture.upload,
}));
import { KnowledgeBasePage } from "./KnowledgeBasePage";
import { KnowledgeBaseDetailPage } from "./KnowledgeBaseDetailPage";
async function openDocumentActions() {
 fireEvent.click(screen.getByRole("button", { name: `Actions for ${fixture.filename}` }));
 await screen.findByRole("menuitem", { name: "Settings" });
}
async function chooseDocumentAction(name: string) {
 await openDocumentActions();
 fireEvent.click(screen.getByRole("menuitem", { name }));
}
function mount(path = "/knowledge-base/kb-a") {
 const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
 const element = () => <QueryClientProvider client={client}><MemoryRouter initialEntries={[path]}><Routes><Route path="/knowledge-base" element={<KnowledgeBasePage />} /><Route path="/knowledge-base/:knowledgeBaseId" element={<KnowledgeBaseDetailPage />} /></Routes></MemoryRouter></QueryClientProvider>;
 const view = render(element());
 return { ...view, refresh: () => view.rerender(element()) };
}
beforeEach(() => { vi.clearAllMocks(); fixture.workspace = "space-a"; fixture.user = "user-a"; fixture.warnings = []; fixture.filename = "policy.md"; fixture.status="FAILED"; fixture.stage=undefined; fixture.hasArtifact=true; fixture.parsingTier=null; fixture.fidelityIssues=[]; fixture.capabilities.mockResolvedValue({ defaultTier: "standard", availableTiers: ["flash", "standard", "advanced"], parserVersion: "4.0.10", officeTier: "flash" }); fixture.cancel.mockResolvedValue({status:"CANCELLED"});fixture.reparse.mockResolvedValue({status:"UPLOADED"});fixture.create.mockResolvedValue({ id: "kb-a" }); fixture.update.mockResolvedValue({}); fixture.preview.mockResolvedValue({ chunkCount: 1, chunks: [{ text: "Policy content", title: "Policy", headingPath: ["Policy"], ordinal: 0, location: { heading: "Policy", charStart: 0, charEnd: 14 } }] }); fixture.index.mockResolvedValue({ status: "READY" }); fixture.grants.mockResolvedValue({}); fixture.upload.mockResolvedValue({}); });
afterEach(cleanup);
describe("knowledge base flow", () => {
 it.each(["READY", "FAILED", "CANCELLED", "NEEDS_REBUILD"])("shows only %s even if the last task stage was publishing", async (status) => {
  fixture.status=status; fixture.stage="Publishing";
  mount(); await screen.findByText("policy.md");
  const statusCell=screen.getAllByRole("row")[1].querySelectorAll("td")[3];
  expect(statusCell.textContent).toBe(pages.knowledgeLibrary.statuses[status as keyof typeof pages.knowledgeLibrary.statuses]);
  expect(statusCell.querySelectorAll("p")).toHaveLength(1);
  expect(statusCell.querySelector('[data-slot="badge"]')).toBeNull();
 });
 it.each(["Parsing", "Chunking", "Embedding", "Publishing"] as const)("identifies the queued %s stage without showing two statuses", async (stage) => {
  fixture.status="QUEUED"; fixture.stage=stage;
  mount(); await screen.findByText("policy.md");
  const statusCell=screen.getAllByRole("row")[1].querySelectorAll("td")[3];
  expect(statusCell.textContent).toBe(pages.knowledgeLibrary.queuedStages[stage]);
  expect(statusCell.querySelectorAll("p")).toHaveLength(1);
 });
 it("shows queued when no stage is available", async () => {
  fixture.status="QUEUED"; fixture.stage=null;
  mount(); await screen.findByText("policy.md");
  expect(screen.getAllByRole("row")[1].querySelectorAll("td")[3].textContent).toBe("Queued");
 });
 it("keeps parsing quality separate from inherited automatic splitting before and after parsing", async () => {
  fixture.filename="guideline.pdf"; fixture.status="QUEUED"; fixture.hasArtifact=false;
  const view=mount(); await screen.findByText("guideline.pdf");
  expect(screen.getByRole("columnheader",{name:"Parsing quality"})).toBeTruthy();
  expect(screen.getByRole("columnheader",{name:"Splitting"})).toBeTruthy();
  await waitFor(()=>expect(within(screen.getAllByRole("row")[1]).getByText("Standard",{exact:true})).toBeTruthy());
  expect(within(screen.getAllByRole("row")[1]).getByText("Automatic",{exact:true})).toBeTruthy();
  view.unmount(); fixture.status="READY"; fixture.hasArtifact=true; mount(); await screen.findByText("guideline.pdf");
  expect(within(screen.getAllByRole("row")[1]).getByText("Standard",{exact:true})).toBeTruthy();
  expect(within(screen.getAllByRole("row")[1]).getByText("Automatic",{exact:true})).toBeTruthy();
 });
 it("shows the requested PDF tier while parsing and does not guess when defaults are unavailable", async () => {
  fixture.filename="guideline.pdf"; fixture.status="PARSING"; fixture.hasArtifact=false; fixture.parsingTier="advanced";
  const view=mount(); await screen.findByText("guideline.pdf");
  expect(within(screen.getAllByRole("row")[1]).getByText("Advanced",{exact:true})).toBeTruthy();
  view.unmount(); fixture.parsingTier=null; fixture.capabilities.mockRejectedValueOnce(new Error("unavailable"));
  mount(); await screen.findByText("guideline.pdf");
  await waitFor(()=>expect(fixture.capabilities).toHaveBeenCalled());
  expect(within(screen.getAllByRole("row")[1]).getByText("Default",{exact:true})).toBeTruthy();
  expect(screen.queryByText("Standard",{exact:true})).toBeNull();
 });
 it.each(["policy.md","policy.docx","policy.xlsx"])("shows native text for %s", async (filename) => {
  fixture.filename=filename; mount(); await screen.findByText(filename);
  expect(within(screen.getAllByRole("row")[1]).getByText("Native text",{exact:true})).toBeTruthy();
 });
 it("uploads a batch independently and reports successes and named failures", async () => {
  mount(); await screen.findByText("policy.md");
  const first=new File(["first"],"first.pdf"); const failed=new File(["failed"],"failed.pdf");
  const last=new File(["last"],"last.md"); const oversized=new File(["large"],"large.pdf");
  Object.defineProperty(oversized,"size",{value:20*1024*1024+1});
  fixture.upload.mockResolvedValueOnce({status:"QUEUED"}).mockRejectedValueOnce(new Error("Network unavailable")).mockResolvedValueOnce({status:"QUEUED"});
  const input=screen.getByLabelText("Upload") as HTMLInputElement;
  expect(input.multiple).toBe(true);
  fireEvent.change(input,{target:{files:[first,new File(["old"],"old.doc"),failed,oversized,last]}});
  await waitFor(()=>expect(fixture.success).toHaveBeenCalledWith("Uploaded 2 files."));
  expect(fixture.upload.mock.calls.map(call=>call.slice(0,3))).toEqual([["space-a","kb-a",first],["space-a","kb-a",failed],["space-a","kb-a",last]]);
  expect(fixture.error).toHaveBeenCalledWith(expect.stringContaining("3 files could not be uploaded."));
  expect(fixture.error).toHaveBeenCalledWith(expect.stringContaining("old.doc:"));
  expect(fixture.error).toHaveBeenCalledWith(expect.stringContaining("failed.pdf: Network unavailable"));
  expect(fixture.error).toHaveBeenCalledWith(expect.stringContaining("large.pdf:"));
  expect(input.value).toBe("");
  await waitFor(()=>expect(input.disabled).toBe(false));
  fireEvent.change(input,{target:{files:[first]}});
  await waitFor(()=>expect(fixture.success).toHaveBeenCalledWith("Uploaded 1 file."));
  expect(fixture.upload).toHaveBeenCalledTimes(4);
 });
 it.each(["workspace","user"] as const)("stops scheduling a batch when the active %s changes", async (scope) => {
  let complete!: (value: {status:string})=>void;
  fixture.upload.mockImplementationOnce(()=>new Promise(resolve=>{complete=resolve;}));
  const view=mount(); await screen.findByText("policy.md");
  fireEvent.change(screen.getByLabelText("Upload"),{target:{files:[new File(["one"],"one.pdf"),new File(["two"],"two.pdf")]}});
  await waitFor(()=>expect(fixture.upload).toHaveBeenCalledTimes(1));
  fixture[scope]=scope==="workspace"?"space-b":"user-b"; view.refresh();
  complete({status:"QUEUED"});
  await waitFor(()=>expect((screen.getByLabelText("Upload") as HTMLInputElement).disabled).toBe(false));
  expect(fixture.upload).toHaveBeenCalledTimes(1);
  expect(fixture.success).not.toHaveBeenCalled(); expect(fixture.error).not.toHaveBeenCalled();
 });
 it.each(["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"])("shows %s with cancellation and disables settings until processing finishes", async (status) => {
  fixture.status = status; mount(); await screen.findByText("policy.md");
  await openDocumentActions();
  expect(screen.getByRole("menuitem", { name: "Settings" }).getAttribute("aria-disabled")).toBe("true");
  const statusCell = screen.getAllByRole("row")[1].querySelectorAll("td")[3];
  expect(statusCell.querySelector('[data-slot="badge"]')).toBeNull();
  const firstLine = status === "QUEUED" ? pages.knowledgeLibrary.queuedStages.Parsing : pages.knowledgeLibrary.statuses[status as keyof typeof pages.knowledgeLibrary.statuses];
  expect(within(statusCell).getByText(firstLine, { exact: true })).toBeTruthy();
  expect(statusCell.querySelectorAll("p")).toHaveLength(1);
  fireEvent.click(screen.getByRole("menuitem", { name: "Cancel" }));
  await waitFor(() => expect(fixture.cancel).toHaveBeenCalledWith("space-a", "kb-a", "doc-a"));
  await waitFor(() => expect(fixture.success).toHaveBeenCalledWith("Processing cancelled."));
 });
 it("allows a cancelled document to retry and queues processing without claiming completion", async () => {
  fixture.status = "CANCELLED"; fixture.index.mockResolvedValueOnce({ status: "QUEUED" }); mount(); await screen.findByText("Cancelled");
  await chooseDocumentAction("Retry");
  await waitFor(() => expect(fixture.index).toHaveBeenCalledWith("space-a", "kb-a", "doc-a", true));
  await waitFor(() => expect(fixture.success).toHaveBeenCalledWith(pages.knowledgeLibrary.processingStarted));
 });
 it("shows a recorded parser failure with retry and no user audit or reprocessing actions", async () => {
  fixture.filename="guideline.pdf";fixture.status="REVIEW_REQUIRED";mount();await screen.findByText("guideline.pdf");
  await openDocumentActions();expect(screen.getByRole("menuitem",{name:"Retry"})).toBeTruthy();expect(screen.queryByRole("button",{name:"Reprocess page images"})).toBeNull();
  expect(screen.queryByText("Source review required; index not published")).toBeNull();expect(fixture.success).not.toHaveBeenCalled();
 });
 it("previews unchanged settings without invalidating the existing index", async () => { mount(); await screen.findByText("policy.md"); await chooseDocumentAction("Settings"); expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true); fireEvent.click(screen.getByRole("button", { name: "Preview" })); await screen.findByText("Policy content"); expect(fixture.preview).toHaveBeenCalledWith("space-a", "kb-a", "doc-a"); expect(fixture.update).not.toHaveBeenCalled(); expect(fixture.index).not.toHaveBeenCalled(); });
 it("inherits platform Standard and saves explicit per-document parsing quality", async () => { fixture.filename="synthetic.pdf"; mount(); await screen.findByText("synthetic.pdf"); await chooseDocumentAction("Settings"); await screen.findByRole("option",{name:"Advanced"}); expect(screen.getByRole("option",{name:"Use platform default (Standard)"})).toBeTruthy(); fireEvent.change(screen.getByLabelText("Parsing quality"),{target:{value:"advanced"}}); fireEvent.click(screen.getByRole("button",{name:"Save"})); await waitFor(()=>expect(fixture.update).toHaveBeenCalledWith("space-a","kb-a","doc-a",null,"advanced",null)); });
 it("does not enable parsing preview when capabilities are unavailable", async () => {fixture.filename="synthetic.pdf"; fixture.capabilities.mockRejectedValueOnce(new Error("MinerU is unavailable")); mount(); await screen.findByText("synthetic.pdf");await chooseDocumentAction("Settings");await screen.findByText("MinerU is unavailable");expect((screen.getByRole("button",{name:"Preview"}) as HTMLButtonElement).disabled).toBe(true);});

 it("shows parsing limitations on both the document and its preview", async () => { fixture.warnings = ["Spreadsheet formula values use cached results"]; fixture.preview.mockResolvedValueOnce({ chunkCount: 0, chunks: [], warnings: ["Some PDF pages contain no text; OCR is unsupported"] }); mount(); await screen.findByRole("button", { name: "Warnings for policy.md" }); expect(screen.queryByText("Spreadsheet formula values use cached results")).toBeNull(); await chooseDocumentAction("Settings"); fireEvent.click(screen.getByRole("button", { name: "Preview" })); await screen.findByText("Some PDF pages contain no text; OCR is unsupported"); });
 it("does not reuse private document cache after changing accounts", async () => { const view = mount(); await screen.findByText("policy.md"); fixture.user = "user-b"; view.refresh(); expect(screen.queryByText("policy.md")).toBeNull(); await screen.findByText("other-policy.md"); });
 it("does not navigate to an old workspace after a pending create completes", async () => { let complete!: (value: { id: string }) => void; fixture.create.mockImplementationOnce(() => new Promise((resolve) => { complete = resolve; })); const view = mount("/knowledge-base"); fireEvent.click(screen.getByRole("button", { name: "Create" })); fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Policies" } }); fireEvent.click(screen.getAllByRole("button", { name: "Create" }).at(-1)!); await waitFor(() => expect(fixture.create).toHaveBeenCalled()); fixture.workspace = "space-b"; view.refresh(); complete({ id: "kb-a" }); await waitFor(() => expect(screen.getByText("No knowledge bases. Create one to upload documents.")).toBeTruthy()); expect(screen.queryByText("policy.md")).toBeNull(); expect(fixture.success).not.toHaveBeenCalled(); });
 it("creates a product knowledge base and opens its document page", async () => { mount("/knowledge-base"); fireEvent.click(screen.getByRole("button", { name: "Create" })); fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Policies" } }); fireEvent.click(screen.getAllByRole("button", { name: "Create" }).at(-1)!); await waitFor(() => expect(fixture.create).toHaveBeenCalledWith("space-a", "Policies", "")); await screen.findByText("policy.md"); });
 it("shows indexing errors and retries the document rather than reporting mock success", async () => { mount(); await screen.findByRole("button", { name: "Warnings for policy.md" }); expect(screen.queryByText("Embedding unavailable")).toBeNull(); await chooseDocumentAction("Retry"); await waitFor(() => expect(fixture.index).toHaveBeenCalledWith("space-a", "kb-a", "doc-a", true)); });
 it.each(["PARTIAL_FAILED", "REVIEW_REQUIRED"])("keeps the legacy %s status localized and retryable", async (status) => { fixture.status = status; mount(); await screen.findByText("policy.md"); expect(screen.getByText("Failed", { exact: true })).toBeTruthy(); await openDocumentActions();expect(screen.getByRole("menuitem", { name: "Retry" }).getAttribute("aria-disabled")).not.toBe("true"); });
 it("persists a document-specific strategy before preview and leaves others untouched", async () => { mount(); await screen.findByText("policy.md"); await chooseDocumentAction("Settings"); fireEvent.click(screen.getByRole("switch")); fireEvent.click(screen.getByText("Adjust chunk parameters")); fireEvent.change(screen.getByLabelText("Maximum characters per chunk"), { target: { value: "600" } }); fireEvent.click(screen.getByRole("button", { name: "Save and preview" })); await screen.findByText("Policy content"); expect(screen.getByText("Chunk 1 · Policy")).toBeTruthy(); expect(fixture.update).toHaveBeenCalledWith("space-a", "kb-a", "doc-a", { mode: "AUTO", maxChars: 600, overlapChars: 120, headingDepth: 2 }); expect(fixture.preview).toHaveBeenCalledWith("space-a", "kb-a", "doc-a"); expect(fixture.index).not.toHaveBeenCalled(); });
 it("grants nothing by default and requires explicit selection and save", async () => { mount(); await screen.findByText("policy.md"); fireEvent.click(screen.getByRole("button", { name: "Authorize Agents" })); const toggle = await screen.findByRole("switch", { name: "Main Agent" }); expect((toggle as HTMLInputElement).checked).toBe(false); expect(fixture.grants).not.toHaveBeenCalled(); fireEvent.click(toggle); fireEvent.click(screen.getByRole("button", { name: "Save" })); await waitFor(() => expect(fixture.grants).toHaveBeenCalledWith("space-a", "kb-a", ["main-agent"])); });
 it("rejects unsupported files before upload and sends the original accepted file", async () => { mount(); await screen.findByText("policy.md"); const input = screen.getByLabelText("Upload"); fireEvent.change(input, { target: { files: [new File(["old"], "old.doc")] } }); expect(fixture.upload).not.toHaveBeenCalled(); await waitFor(() => expect(fixture.error).toHaveBeenCalled()); const file = new File(["# Policy"], "policy.md"); fireEvent.change(input, { target: { files: [file] } }); await waitFor(() => expect(fixture.upload).toHaveBeenCalledWith("space-a", "kb-a", file)); });
});
