// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ChatWindow } from "@eliteforge/agent-ui-chat-react";
import { readFileSync } from "node:fs";
const componentsCss = readFileSync("src/shared/theme/components.css", "utf8");
afterEach(cleanup);
it("keeps floating controls as icons while preserving accessible names", () => {
  const style=document.createElement("style"); style.textContent=componentsCss.slice(componentsCss.indexOf("/* Transcript actions")); document.head.append(style);
  try {
    render(<ChatWindow draft="" messages={[]} onDraftChange={vi.fn()} onSend={vi.fn()} title="Platform main Dude" state={{open:true,collapsed:false,x:0,y:0,width:400,height:600}} onStateChange={vi.fn()} />);
    for (const name of ["Collapse chat", "Close chat"]) {
      const control=screen.getByRole("button", {name});
      expect(control.textContent).toBe("");
      expect(control.querySelector("svg")).toBeTruthy();
      expect(getComputedStyle(control.querySelector("svg")!).display).not.toBe("none");
      expect(getComputedStyle(control).minWidth).not.toBe("max-content");
    }
    expect(screen.getByRole("button", {name:"Resize chat window"}).textContent).toBe("");
    expect(getComputedStyle(screen.getByText("Platform main Dude")).whiteSpace).toBe("nowrap");
  } finally {style.remove();}
});
