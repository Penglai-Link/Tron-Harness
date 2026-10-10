// @vitest-environment jsdom
import {
  useState,
  type ReactNode,
  type SelectHTMLAttributes,
  type InputHTMLAttributes,
} from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { copyLoopDesign } from "../model/builtinLoops";
import { validateLoop } from "../model/loopValidation";
import type { BranchCondition } from "../model/LoopModel";
import { BranchEditor } from "./BranchEditor";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock("@/components/forms", () => ({
  Field: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  SelectInput: ({
    onValueChange,
    ...props
  }: SelectHTMLAttributes<HTMLSelectElement> & {
    onValueChange: (value: string) => void;
  }) => (
    <select
      {...props}
      onChange={(event) => onValueChange(event.currentTarget.value)}
    />
  ),
  TextInput: (props: InputHTMLAttributes<HTMLInputElement>) => (
    <input {...props} />
  ),
}));
vi.mock("@/components/ui/select", () => ({
  SelectItem: ({ children, value }: { children: ReactNode; value: string }) => (
    <option value={value}>{children}</option>
  ),
}));
afterEach(cleanup);
const invalid = vi.fn();
function Harness() {
  const design = {
    loopCode: "approval-flow",
    name: "Approval flow",
    kind: "Dag" as const,
    version: 1,
    updatedAt: 0,
    maxParallelNodes: 2,
    edges: [
      { from: "solution", to: "decision" },
      { from: "risks", to: "decision" },
      { from: "decision", to: "route" },
      { from: "route", to: "action" },
      { from: "route", to: "clarify" },
    ],
    nodes: [
      { id: "solution", kind: "Model" as const, instructions: "Solution", join: "Selected" as const, condition: null },
      { id: "risks", kind: "Model" as const, instructions: "Risks", join: "Selected" as const, condition: null },
      { id: "decision", kind: "Model" as const, instructions: "Decision", join: "All" as const, condition: null },
      { id: "route", kind: "Branch" as const, instructions: "", join: "Selected" as const, condition: { sourceNode: "decision", pointer: "/approved", equals: true, matchTarget: "action", elseTarget: "clarify" } },
      { id: "action", kind: "Model" as const, instructions: "Action", join: "All" as const, condition: null },
      { id: "clarify", kind: "Model" as const, instructions: "Clarify", join: "All" as const, condition: null },
    ],
  };
  const [condition, setCondition] = useState<BranchCondition>(
    design.nodes.find((n) => n.kind === "Branch")!.condition!,
  );
  return (
    <>
      <BranchEditor
        condition={condition}
        design={design}
        onChange={setCondition}
        onInvalid={invalid}
      />
      <output aria-label="saved-condition">{JSON.stringify(condition)}</output>
    </>
  );
}
const saved = () =>
  JSON.parse(
    screen.getByLabelText("saved-condition").textContent!,
  ) as BranchCondition;
describe("Branch configuration", () => {
  it("edits named and nested fields without losing a partial slash", () => {
    render(<Harness />);
    const field = screen.getByLabelText("agentLoops.pointer");
    expect((field as HTMLInputElement).value).toBe("approved");
    fireEvent.change(field, { target: { value: "status" } });
    expect(saved().pointer).toBe("/status");
    fireEvent.change(field, { target: { value: "/" } });
    expect((field as HTMLInputElement).value).toBe("/");
    fireEvent.change(field, { target: { value: "/result/approved" } });
    expect(saved().pointer).toBe("/result/approved");
  });
  it("stores typed values while text needs no JSON quotes", () => {
    render(<Harness />);
    fireEvent.change(screen.getByLabelText("agentLoops.equals"), {
      target: { value: "false" },
    });
    expect(saved().equals).toBe(false);
    fireEvent.change(screen.getByLabelText("agentLoops.valueType"), {
      target: { value: "string" },
    });
    fireEvent.change(screen.getByLabelText("agentLoops.equals"), {
      target: { value: "approved" },
    });
    expect(saved().equals).toBe("approved");
    fireEvent.change(screen.getByLabelText("agentLoops.valueType"), {
      target: { value: "number" },
    });
    fireEvent.change(screen.getByLabelText("agentLoops.equals"), {
      target: { value: "1.5" },
    });
    expect(saved().equals).toBe(1.5);
    fireEvent.change(screen.getByLabelText("agentLoops.equals"), {
      target: { value: "" },
    });
    expect(invalid).toHaveBeenLastCalledWith(true);
    expect(saved().equals).toBe(1.5);
    fireEvent.change(screen.getByLabelText("agentLoops.valueType"), {
      target: { value: "null" },
    });
    expect(saved().equals).toBeNull();
    expect(invalid).toHaveBeenLastCalledWith(false);
  });
  it("copies an authored design into an independent draft", () => {
    const source = {
      loopCode: "approval-flow",
      name: "Approval flow",
      kind: "Workflow" as const,
      version: 1,
      updatedAt: 1,
      maxParallelNodes: 1,
      edges: [],
      nodes: [{ id: "stage-1", kind: "Model" as const, instructions: "Review", join: "Selected" as const, condition: null }],
    };
    const copy = copyLoopDesign(source, "My flow");
    copy.loopCode = "my-flow";
    expect(validateLoop(copy)).toBeNull();
    expect(copy.version).toBe(0);
    copy.nodes[0].instructions = "Edited";
    expect(source.nodes[0].instructions).toBe("Review");
  });
});
