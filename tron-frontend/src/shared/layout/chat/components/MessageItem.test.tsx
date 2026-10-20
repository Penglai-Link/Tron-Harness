// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChatSession } from "@/domain/chat/model";
import MessageItem from "./MessageItem";

const actions = vi.hoisted(() => ({
  archive: vi.fn(),
  delete: vi.fn(),
  update: vi.fn(),
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock("@/domain/chat/hooks", () => ({
  archiveChatSession: actions.archive,
  deleteArchivedChatSession: actions.delete,
  updateChatSession: actions.update,
}));
vi.mock("@/components/ui/toast", () => ({ useToast: () => ({ success: actions.toastSuccess, error: actions.toastError }) }));
vi.mock("react-i18next", async (importOriginal) => ({
  ...await importOriginal<typeof import("react-i18next")>(),
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock("@/components/ui/dropdown-menu", () => ({
  DropdownMenu: ({ children }: { children: ReactNode }) => <>{children}</>,
  DropdownMenuTrigger: ({ render }: { render: ReactNode }) => render,
  DropdownMenuContent: ({ children }: { children: ReactNode }) => <div role="menu">{children}</div>,
  DropdownMenuItem: ({ children, onClick, disabled }: { children: ReactNode; onClick: () => void; disabled?: boolean }) => <button role="menuitem" disabled={disabled} onClick={onClick}>{children}</button>,
  DropdownMenuSeparator: () => <hr />,
}));
vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: ReactNode; open: boolean }) => open ? <>{children}</> : null,
  DialogContent: ({ children }: { children: ReactNode }) => <div role="dialog">{children}</div>,
  DialogFooter: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DialogHeader: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DialogTitle: ({ children }: { children: ReactNode }) => <h2>{children}</h2>,
}));
vi.mock("@/components/confirm-dialog", () => ({
  ConfirmDialog: ({ open, onConfirm, confirmText }: { open: boolean; onConfirm: () => void; confirmText: string }) =>
    open ? <button onClick={onConfirm}>{confirmText}</button> : null,
}));

const session: ChatSession = {
  id: 1,
  sessionCode: "SESSION-1",
  workspaceCode: "workspace",
  title: "Original title",
  status: "ACTIVE",
  createdAt: "2026-10-20T00:00:00Z",
  updatedAt: "2026-10-20T00:00:00Z",
};

function renderItem(overrides: Partial<React.ComponentProps<typeof MessageItem>> = {}) {
  const onTogglePin = vi.fn();
  const onRemovePin = vi.fn();
  render(<MemoryRouter><MessageItem
    active={false}
    onNavigate={vi.fn()}
    onRemovePin={onRemovePin}
    onTogglePin={onTogglePin}
    pinned={false}
    running={false}
    session={session}
    {...overrides}
  /></MemoryRouter>);
  return { onTogglePin, onRemovePin };
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("chat sidebar session actions", () => {
  it("pins and renames the selected session", async () => {
    const { onTogglePin } = renderItem();
    actions.update.mockResolvedValue({ ...session, title: "New title" });
    fireEvent.click(screen.getByRole("menuitem", { name: "shell:chatSidebar.session.pin" }));
    expect(onTogglePin).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("menuitem", { name: "shell:chatSidebar.session.rename" }));
    fireEvent.change(screen.getByRole("textbox", { name: "shell:chatSidebar.session.title" }), { target: { value: " New title " } });
    fireEvent.click(screen.getByRole("button", { name: "shell:chatSidebar.session.save" }));
    await waitFor(() => expect(actions.update).toHaveBeenCalledWith("SESSION-1", { title: "New title" }));
  });

  it("archives before permanently deleting and removes the pin", async () => {
    const order: string[] = [];
    actions.archive.mockImplementation(async () => { order.push("archive"); return session; });
    actions.delete.mockImplementation(async () => { order.push("delete"); return session; });
    const { onRemovePin } = renderItem();
    fireEvent.click(screen.getByRole("menuitem", { name: "shell:chatSidebar.session.delete" }));
    expect(actions.archive).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "shell:chatSidebar.session.delete" }));
    await waitFor(() => expect(onRemovePin).toHaveBeenCalledOnce());
    expect(order).toEqual(["archive", "delete"]);
  });

  it("copies the exact session ID", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    renderItem();
    fireEvent.click(screen.getByRole("menuitem", { name: "shell:chatSidebar.session.copyId" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("SESSION-1"));
  });
});
