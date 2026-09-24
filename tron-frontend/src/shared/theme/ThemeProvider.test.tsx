// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ThemeProvider, useTheme } from "./ThemeProvider";
import { skinApi } from "./skinApi";
import type { Skin } from "./Skin";
import light from "./builtin/light.json";
import matrix from "./builtin/matrix-dark.json";

vi.mock("@/store/authStore", () => ({ useAuthStore: () => "authenticated" }));
vi.mock("./skinApi", () => ({ skinApi: { list: vi.fn(), active: vi.fn(), activate: vi.fn(), install: vi.fn(), watch: vi.fn(() => () => {}) } }));
let context: ReturnType<typeof useTheme>;
function Probe() {
  context = useTheme();
  return <span data-testid="active">{context.activeSkin.id}</span>;
}
beforeEach(() => {
  localStorage.clear();
  vi.mocked(skinApi.list).mockResolvedValue([light, matrix] as Skin[]);
  vi.mocked(skinApi.active).mockResolvedValue({ skin: light as Skin });
  vi.mocked(skinApi.activate).mockImplementation(async () => {
    vi.mocked(skinApi.active).mockResolvedValue({ skin: matrix as Skin });
    return { skin: matrix as Skin };
  });
});
afterEach(() => { cleanup(); vi.clearAllMocks(); });

it("keeps the current skin and cleans prepared resources when persistence fails", async () => {
  render(<ThemeProvider><Probe /></ThemeProvider>);
  await waitFor(() => expect(context.loading).toBe(false));
  vi.mocked(skinApi.activate).mockRejectedValueOnce(new Error("write failed"));
  await act(async () => { await expect(context.selectSkin("matrix-dark")).rejects.toThrow("write failed"); });
  expect(screen.getByTestId("active").textContent).toBe("light");
  expect(document.documentElement.dataset.skin).toBe("light");
  expect(document.querySelectorAll("style[data-tron-skin]")).toHaveLength(1);
});

it("does not let an older refresh overwrite a completed selection", async () => {
  render(<ThemeProvider><Probe /></ThemeProvider>);
  await waitFor(() => expect(context.loading).toBe(false));
  let resolveOld!: (value: { skin: Skin }) => void;
  vi.mocked(skinApi.active).mockImplementationOnce(() => new Promise((resolve) => { resolveOld = resolve; }));
  let refresh!: Promise<void>;
  act(() => { refresh = context.refreshSkins(); });
  await act(async () => { await context.selectSkin("matrix-dark"); });
  await act(async () => { resolveOld({ skin: light as Skin }); await refresh; });
  expect(screen.getByTestId("active").textContent).toBe("matrix-dark");
  expect(document.querySelectorAll("style[data-tron-skin]")).toHaveLength(1);
});

it("cleans the applied stylesheet on unmount", async () => {
  const view = render(<ThemeProvider><Probe /></ThemeProvider>);
  await waitFor(() => expect(context.loading).toBe(false));
  view.unmount();
  expect(document.querySelectorAll("style[data-tron-skin]")).toHaveLength(0);
});

it("keeps installed choices available when the active skin has a broken asset", async () => {
  const broken = { ...light, id: "broken", resources: { background: "https://example.invalid/broken.png", font: null } } as Skin;
  vi.mocked(skinApi.list).mockResolvedValue([light, matrix, broken] as Skin[]);
  vi.mocked(skinApi.active).mockResolvedValue({ skin: broken });
  vi.stubGlobal("Image", class {
    onerror?: () => void;
    set src(_value: string) { queueMicrotask(() => this.onerror?.()); }
  });
  render(<ThemeProvider><Probe /></ThemeProvider>);
  await waitFor(() => expect(context.loading).toBe(false));
  expect(context.skins.map((skin) => skin.id)).toEqual(["light", "matrix-dark", "broken"]);
  expect(context.error).toBeTruthy();
  await act(async () => { await context.selectSkin("matrix-dark"); });
  expect(context.activeSkin.id).toBe("matrix-dark");
  vi.unstubAllGlobals();
});
