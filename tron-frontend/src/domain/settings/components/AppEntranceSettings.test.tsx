// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AppEntranceSettings } from "./AppEntranceSettings";

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
afterEach(() => { cleanup(); localStorage.clear(); });

it("defaults off and restores both enabled and disabled choices after remount", () => {
  window.PointerEvent ??= MouseEvent as typeof PointerEvent;
  const name = "settingsAppearance.appEntranceAnimation";
  const first = render(<AppEntranceSettings />);
  expect(screen.getByRole("switch", { name }).getAttribute("aria-checked")).toBe("false");
  fireEvent.click(screen.getByRole("switch", { name }));
  first.unmount();
  const second = render(<AppEntranceSettings />);
  expect(screen.getByRole("switch", { name }).getAttribute("aria-checked")).toBe("true");
  fireEvent.click(screen.getByRole("switch", { name }));
  second.unmount();
  render(<AppEntranceSettings />);
  expect(screen.getByRole("switch", { name }).getAttribute("aria-checked")).toBe("false");
});
