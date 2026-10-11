// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
// Exercise the application policy independently of floating layout (verified in Chrome).
vi.mock("@base-ui/react/tooltip", async () => {
  const React = await import("react");
  const Context = React.createContext({ open: false, change: (_open: boolean, _reason: string) => {} });
  return { Tooltip: {
    Root: ({ open, onOpenChange, children }: any) => <Context.Provider value={{ open, change: (next, reason) => {
      const details = { reason, isCanceled: false, cancel() { this.isCanceled = true; } };
      onOpenChange(next, details);
    } }}>{children}</Context.Provider>,
    Provider: ({ children }: any) => children,
    Trigger: ({ delay: _delay, closeDelay: _closeDelay, children, ...props }: any) => {
      const context = React.useContext(Context);
      return <button {...props} onFocus={() => context.change(true, "trigger-focus")} onClick={() => context.change(true, "trigger-press")}
        onMouseEnter={() => context.change(true, "trigger-hover")} onMouseLeave={() => context.change(false, "trigger-hover")}>{children}</button>;
    },
    Portal: ({ children }: any) => children,
    Positioner: ({ children }: any) => children,
    Popup: ({ children, ...props }: any) => React.useContext(Context).open ? <div {...props}>{children}</div> : null,
    Arrow: () => null,
  } };
});
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./tooltip";

function Hint({ name }: { name: string }) {
  return <Tooltip><TooltipTrigger delay={0} closeDelay={0} title="duplicate">{name}</TooltipTrigger><TooltipContent>{name} content</TooltipContent></Tooltip>;
}

afterEach(cleanup);
describe("hover-only tooltips", () => {
  it("ignores focus and click and removes native title", async () => {
    render(<Hint name="First" />);
    const trigger = screen.getByRole("button");
    fireEvent.focus(trigger);
    fireEvent.click(trigger);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.queryByRole("tooltip")).toBeNull();
    expect(trigger.hasAttribute("title")).toBe(false);
  });
  it("opens on hover and replaces the previous tooltip across providers", async () => {
    render(<><TooltipProvider><Hint name="First" /></TooltipProvider><TooltipProvider><Hint name="Second" /></TooltipProvider></>);
    fireEvent.mouseEnter(screen.getByRole("button", { name: "First" }));
    await waitFor(() => expect(screen.getByRole("tooltip").textContent).toContain("First content"));
    fireEvent.mouseEnter(screen.getByRole("button", { name: "Second" }));
    await waitFor(() => {
      expect(screen.getAllByRole("tooltip")).toHaveLength(1);
      expect(screen.getByRole("tooltip").textContent).toContain("Second content");
    });
  });
  it("releases the active hint when its page unmounts", async () => {
    const view = render(<Hint name="First" />);
    fireEvent.mouseEnter(screen.getByRole("button"));
    await screen.findByRole("tooltip");
    view.unmount();
    render(<Hint name="Second" />);
    expect(screen.queryByRole("tooltip")).toBeNull();
    fireEvent.mouseEnter(screen.getByRole("button"));
    await waitFor(() => expect(screen.getByRole("tooltip").textContent).toContain("Second content"));
  });
});
