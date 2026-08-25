// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EditableTabs } from "./index";
import { OperateType, type EditableTab } from "./types";

const initialTabs: EditableTab[] = [
  { key: "one", label: "One", children: <p>One content</p> },
  { key: "two", label: "Two", children: <p>Two content</p>, closable: false },
];

describe("EditableTabs", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the initial active tab and supports direct additions", async () => {
    const onChange = vi.fn();

    render(
      <EditableTabs
        createTab={() => ({ key: "three", label: "Three", children: <p>Three content</p> })}
        items={initialTabs}
        onChange={onChange}
      />,
    );

    expect(screen.getByText("One content")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Add tab" }));

    await waitFor(() => expect(screen.getByText("Three content")).toBeTruthy());
    expect(onChange).toHaveBeenLastCalledWith("three");
  });

  it("supports a custom trigger and hides the built-in add button", async () => {
    render(
      <EditableTabs
        createTab={() => ({ key: "three", label: "Three", children: <p>Three content</p> })}
        hideAdd
        items={initialTabs}
        renderAddTrigger={({ onAdd }) => (
          <button onClick={onAdd} type="button">
            Add view
          </button>
        )}
      />,
    );

    expect(screen.queryByRole("button", { name: "Add tab" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Add view" }));
    await waitFor(() => expect(screen.getByText("Three content")).toBeTruthy());
  });

  it("closes the active tab and selects the next tab", async () => {
    const onChange = vi.fn();
    const onEdit = vi.fn();

    render(<EditableTabs items={initialTabs} onChange={onChange} onEdit={onEdit} />);
    fireEvent.click(screen.getAllByRole("button", { name: "Close tab" })[0]);

    await waitFor(() => expect(screen.getByText("Two content")).toBeTruthy());
    expect(onEdit).toHaveBeenCalledWith(
      OperateType.Remove,
      expect.objectContaining({ key: "one" }),
    );
    expect(onChange).toHaveBeenLastCalledWith("two");
  });

  it("closes a requested tab through the shared close flow", async () => {
    const onEdit = vi.fn();
    const { rerender } = render(<EditableTabs items={initialTabs} onEdit={onEdit} />);

    rerender(
      <EditableTabs
        closeTabRequest={{ id: 1, key: "one" }}
        items={initialTabs}
        onEdit={onEdit}
      />,
    );

    await waitFor(() => expect(screen.getByText("Two content")).toBeTruthy());
    expect(onEdit).toHaveBeenCalledWith(
      OperateType.Remove,
      expect.objectContaining({ key: "one" }),
    );
  });

  it("does not render a close action for a non-closable tab", () => {
    render(<EditableTabs defaultActiveKey="two" items={initialTabs} />);

    expect(screen.getByText("Two content")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Close tab" })).toHaveLength(1);
  });

  it("keeps each tab content mounted while switching tabs", () => {
    render(
      <EditableTabs
        items={[
          {
            key: "one",
            label: "One",
            children: <input aria-label="One input" defaultValue="" />,
          },
          {
            key: "two",
            label: "Two",
            children: <p>Two content</p>,
          },
        ]}
      />,
    );

    const input = screen.getByRole("textbox", { name: "One input" });
    fireEvent.change(input, { target: { value: "draft command" } });
    fireEvent.click(screen.getByRole("tab", { name: "Two" }));
    fireEvent.click(screen.getByRole("tab", { name: "One" }));

    expect((input as HTMLInputElement).value).toBe("draft command");
  });
});
