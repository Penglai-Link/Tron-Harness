// @vitest-environment jsdom
import { useLayoutEffect } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useNavigate } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { HomeArrivalVeil, useAwakeningBackdrop } from "./HomeArrivalVeil";
vi.mock("./CodeRain", () => ({ CodeRain: () => <canvas data-testid="rain" /> }));
afterEach(() => { cleanup(); vi.useRealTimers(); });
function Guide() {
  const { showRain, beginArrival } = useAwakeningBackdrop();
  const navigate = useNavigate();
  useLayoutEffect(() => showRain(true), [showRain]);
  return <button onClick={() => { beginArrival(false); navigate("/workspace"); }}>Enter</button>;
}
it("keeps the exact live rain canvas through navigation, then removes it after home is revealed", async () => {
  vi.useFakeTimers();
  render(<MemoryRouter initialEntries={["/awakening"]}><HomeArrivalVeil><Routes>
    <Route path="/awakening" element={<Guide />} />
    <Route path="/workspace" element={<div>Home</div>} />
  </Routes></HomeArrivalVeil></MemoryRouter>);
  const rain = screen.getByTestId("rain");
  fireEvent.click(screen.getByText("Enter"));
  expect(screen.getByText("Home")).toBeTruthy();
  expect(screen.getByTestId("rain")).toBe(rain);
  expect(rain.parentElement?.dataset.destination).toBe("true");
  await act(() => vi.advanceTimersByTimeAsync(50));
  expect(rain.parentElement?.dataset.phase).toBe("reveal");
  await act(() => vi.advanceTimersByTimeAsync(2700));
  expect(screen.getByTestId("rain")).toBe(rain);
  await act(() => vi.advanceTimersByTimeAsync(350));
  expect(screen.queryByTestId("rain")).toBeNull();
});

it("reveals home without mounting code rain for an initialized user", async () => {
  vi.useFakeTimers();
  function Login() {
    const { beginArrival } = useAwakeningBackdrop();
    const navigate = useNavigate();
    return <button onClick={() => { beginArrival(false, false); navigate("/workspace"); }}>Login</button>;
  }
  const { container } = render(<MemoryRouter initialEntries={["/profiles"]}><HomeArrivalVeil><Routes>
    <Route path="/profiles" element={<Login />} />
    <Route path="/workspace" element={<div>Home</div>} />
  </Routes></HomeArrivalVeil></MemoryRouter>);
  fireEvent.click(screen.getByText("Login"));
  expect(screen.queryByTestId("rain")).toBeNull();
  expect(container.querySelector(".awakening-home-veil")?.getAttribute("data-phase")).toBe("cover");
  await act(() => vi.advanceTimersByTimeAsync(50));
  expect(container.querySelector(".awakening-home-veil")?.getAttribute("data-phase")).toBe("reveal");
  expect(screen.queryByTestId("rain")).toBeNull();
  await act(() => vi.advanceTimersByTimeAsync(750));
  expect(container.querySelector(".awakening-home-veil")).toBeNull();
});
