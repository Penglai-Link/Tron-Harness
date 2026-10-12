// @vitest-environment jsdom
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AgentLoopField } from "./AgentLoopField";

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("../hooks/useLoopDesignList", () => ({ useLoopDesignList: () => ({
  designs: [
    { loopCode: "workflow", name: "Workflow", kind: "Workflow" },
    { loopCode: "react-loop", name: "React loop", kind: "React" },
  ],
  loading: false,
  error: null,
}) }));
vi.mock("@/components/forms", () => ({
  Field: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  SelectInput: ({ children, ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) => <select {...props}>{children}</select>,
}));
vi.mock("@/components/ui/select", () => ({ SelectItem: ({ children, ...props }: React.OptionHTMLAttributes<HTMLOptionElement>) => <option {...props}>{children}</option> }));

afterEach(cleanup);

it("chooses a loop of the requested kind when the selection is empty", async () => {
  const onChange = vi.fn();
  render(<AgentLoopField value="" defaultKind="React" onChange={onChange} />);
  await waitFor(() => expect(onChange).toHaveBeenCalledWith("react-loop"));
});

it("preserves an existing selection", () => {
  const onChange = vi.fn();
  render(<AgentLoopField value="saved-loop" defaultKind="React" onChange={onChange} />);
  expect(onChange).not.toHaveBeenCalled();
});
