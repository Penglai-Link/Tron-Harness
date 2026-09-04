// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
  value: () => null,
});
Object.defineProperty(window, "matchMedia", {
  value: () => ({
    addEventListener() {},
    addListener() {},
    matches: false,
    removeEventListener() {},
    removeListener() {},
  }),
});

function dispatchKey(target: EventTarget, key: string, keyCode: number) {
  const event = new KeyboardEvent("keydown", { bubbles: true, key });
  Object.defineProperties(event, {
    keyCode: { value: keyCode },
    which: { value: keyCode },
  });
  target.dispatchEvent(event);
}

function dispatchInsertText(target: EventTarget, data: string) {
  const event = new InputEvent("input", { bubbles: true, data, inputType: "insertText" });
  Object.defineProperties(event, {
    composed: { value: true },
    inputType: { value: "insertText" },
  });
  target.dispatchEvent(event);
}

describe("xterm Apple Pinyin keyCode=229 patch", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("forwards committed Chinese and following ASCII exactly once", async () => {
    const { Terminal } = await import("@xterm/xterm");
    const terminal = new Terminal();
    const output: string[] = [];
    terminal.onData((data) => output.push(data));
    const host = document.createElement("div");
    document.body.appendChild(host);
    terminal.open(host);

    const textarea = host.querySelector<HTMLTextAreaElement>(".xterm-helper-textarea");
    expect(textarea).toBeInstanceOf(HTMLTextAreaElement);

    // Apple Pinyin can commit text before its synthetic keyCode=229 keydown.
    dispatchInsertText(textarea!, "中");
    dispatchKey(textarea!, "Process", 229);
    dispatchInsertText(textarea!, "文");
    dispatchKey(textarea!, "Process", 229);

    // Switching to ABC must not be replayed by the old delayed textarea diff.
    dispatchKey(textarea!, "a", 65);
    dispatchKey(textarea!, "b", 66);
    dispatchKey(textarea!, "c", 67);
    dispatchInsertText(textarea!, " ");
    dispatchKey(textarea!, "Process", 229);
    dispatchKey(textarea!, "Backspace", 8);

    expect(output.join("")).toBe("中文abc \x7f");
    terminal.dispose();
  });
});
