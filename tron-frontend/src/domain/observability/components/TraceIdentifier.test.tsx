// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { TraceIdentifier } from "./TraceIdentifier";
const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => toast }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("copies the full identifier without activating its navigation action", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  const onClick = vi.fn();
  const value = "11112222333344445555666677778888";
  render(<TraceIdentifier value={value} label="Trace ID" onClick={onClick} />);
  fireEvent.click(screen.getByRole("button", { name: `copyId: ${value}` }));
  await waitFor(() => expect(toast.success).toHaveBeenCalledWith("copied"));
  expect(writeText).toHaveBeenCalledWith(value);
  expect(onClick).not.toHaveBeenCalled();
  writeText.mockRejectedValueOnce(new Error("denied"));
  fireEvent.click(screen.getByRole("button", { name: `copyId: ${value}` }));
  await waitFor(() => expect(toast.error).toHaveBeenCalledWith("copyFailed"));
});
