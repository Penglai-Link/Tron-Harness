// @vitest-environment jsdom
import { render, cleanup } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { LogHighlight } from "./LogHighlight";
afterEach(cleanup);
it("highlights all literal matches without case sensitivity", () => {
  const { container, rerender } = render(<LogHighlight text="a.b A.B axb" keyword="a.b" />);
  expect([...container.querySelectorAll("mark")].map(e => e.textContent)).toEqual(["a.b", "A.B"]);
  expect(container.textContent).toBe("a.b A.B axb");
  rerender(<LogHighlight text="a.b A.B axb" />);
  expect(container.querySelector("mark")).toBeNull();
});
