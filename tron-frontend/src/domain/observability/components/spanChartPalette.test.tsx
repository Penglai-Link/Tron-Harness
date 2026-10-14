// @vitest-environment jsdom
import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { useSpanChartPalette } from "./spanChartPalette";

afterEach(() => {
  delete document.documentElement.dataset.theme;
  document.documentElement.style.removeProperty("--color-on-surface");
  document.documentElement.style.removeProperty("--color-elevated-surface");
});

it("refreshes canvas colors when the active skin changes while mounted", async () => {
  const { result } = renderHook(useSpanChartPalette);
  expect(result.current.dark).toBe(false);
  await act(async () => {
    document.documentElement.dataset.theme = "matrix-dark";
    document.documentElement.style.setProperty("--color-on-surface", "#b5f3c8");
    document.documentElement.style.setProperty("--color-elevated-surface", "#0b1a10");
  });
  await waitFor(() => expect(result.current).toMatchObject({
    dark: true, text: "#b5f3c8", surface: "#0b1a10",
  }));
  await act(async () => {
    document.documentElement.dataset.theme = "light";
    document.documentElement.style.setProperty("--color-on-surface", "#222222");
    document.documentElement.style.setProperty("--color-elevated-surface", "#ffffff");
  });
  await waitFor(() => expect(result.current).toMatchObject({
    dark: false, text: "#222222", surface: "#ffffff",
  }));
});
