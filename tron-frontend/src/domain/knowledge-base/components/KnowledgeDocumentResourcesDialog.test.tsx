// @vitest-environment jsdom
import { pages } from "@/i18n/locales/en-US/pages";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";
import type { ReactNode } from "react";
const fixture = vi.hoisted(() => ({ download: vi.fn() }));
vi.mock("../hooks/useKnowledgeLibraries", () => ({ useKnowledgeUser: () => "owner-a" }));
vi.mock("../api/knowledgeBaseApi", () => ({ downloadKnowledgeResource: fixture.download }));
vi.mock("@/domain/resource/api/resourceApi", () => ({ downloadResourceBinary: vi.fn() }));
vi.mock("@/store/authStore", () => ({ useAuthStore: (select: (state: unknown) => unknown) => select({session:{user:{userCode:"owner-a"}}}) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ error: vi.fn() }) }));
vi.mock("@/components/ui/dialog", () => ({ Dialog: ({ children }: { children: ReactNode }) => <div role="dialog">{children}</div>, DialogContent: ({ children }: { children: ReactNode }) => <div>{children}</div>, DialogHeader: ({ children }: { children: ReactNode }) => <div>{children}</div>, DialogFooter: ({ children }: { children: ReactNode }) => <div>{children}</div>, DialogTitle: ({ children }: { children: ReactNode }) => <h2>{children}</h2>, DialogDescription: ({ children }: { children: ReactNode }) => <p>{children}</p> }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, args?: Record<string, unknown>) => { const value = key.split(".").reduce((node: unknown, item) => (node as Record<string, unknown>)?.[item], pages); return String(value ?? args?.defaultValue ?? key).replace(/{{(\w+)}}/g, (_, k) => String(args?.[k] ?? "")); }, i18n: { resolvedLanguage: "en-US" } }) }));
import { KnowledgeDocumentResourcesDialog } from "./KnowledgeDocumentResourcesDialog";
const doc = { id: "doc-a", fileName: "synthetic.docx", resourceCode: "original-a", parsingArtifact: { imageResourceCodes: ["private-image-a"] } } as KnowledgeDocument;
function mount(onClose = () => undefined) { return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><MemoryRouter><KnowledgeDocumentResourcesDialog libraryId="kb-a" document={doc} onClose={onClose} /></MemoryRouter></QueryClientProvider>); }
beforeEach(() => { vi.clearAllMocks(); Object.defineProperty(URL, "createObjectURL", { value: vi.fn(() => "blob:private-image"), configurable: true }); Object.defineProperty(URL, "revokeObjectURL", { value: vi.fn(), configurable: true }); fixture.download.mockResolvedValue({ blob: new Blob(["synthetic"], { type: "image/png" }) }); });
afterEach(cleanup);
it("loads derived images only through the authenticated knowledge resource API and releases private preview URLs", async () => { const view = mount(); const img = await screen.findByAltText("Image 1 extracted from synthetic.docx"); expect(img.getAttribute("src")).toBe("blob:private-image"); expect(fixture.download).toHaveBeenCalledWith("kb-a", "private-image-a", expect.any(AbortSignal)); view.unmount(); expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:private-image"); });
it("shows resource permission errors and allows an explicit download retry", async () => { fixture.download.mockRejectedValueOnce(new Error("Resource access denied")); mount(); await screen.findByText("Resource access denied"); expect(screen.queryByRole("img")).toBeNull(); fireEvent.click(screen.getByRole("button", { name: "Retry preview" })); await waitFor(() => expect(fixture.download).toHaveBeenCalledTimes(2)); await screen.findByAltText("Image 1 extracted from synthetic.docx"); });

it("previews the original through its library without selecting a workspace", async () => { mount(); await screen.findByAltText("Image 1 extracted from synthetic.docx"); fireEvent.click(screen.getByRole("button", { name: "Preview original" })); await waitFor(() => expect(fixture.download).toHaveBeenCalledWith("kb-a", "original-a", expect.any(AbortSignal))); });
