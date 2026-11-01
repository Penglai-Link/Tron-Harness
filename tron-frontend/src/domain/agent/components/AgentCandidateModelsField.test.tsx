// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AgentCandidateModelsField } from "./AgentCandidateModelsField";
import { fetchModelSelectOptions } from "@/domain/model/api";

vi.mock("@/shared", () => ({ useWorkspaceSpaces: () => ({}) }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/domain/admin/api", () => ({ fetchWorkspaceOptions: vi.fn(async () => [{ workspaceCode: "one" }, { workspaceCode: "two" }]) }));
vi.mock("@/domain/model/api", () => ({ fetchModelSelectOptions: vi.fn(async (workspace: string) => [
  { optionCode: "shared", modelCode: "shared", displayName: "Shared text", modelType: "TEXT", enabled: true, contextWindowTokens: 32000 },
  { optionCode: workspace, modelCode: workspace, displayName: workspace, modelType: "TEXT", enabled: true, contextWindowTokens: 32000 },
]) }));
afterEach(cleanup);

it("loads and deduplicates platform candidates across workspaces without a selected workspace", async () => {
  const onChange = vi.fn();
  render(<AgentCandidateModelsField workspaceCode="$platform" value={["shared"]} onChange={onChange} />);
  expect(await screen.findByRole("checkbox", { name: "Shared text" })).toBeDefined();
  expect(screen.getAllByRole("checkbox", { name: "Shared text" })).toHaveLength(1);
  expect((screen.getByRole("checkbox", { name: "Shared text" }) as HTMLInputElement).checked).toBe(true);
  fireEvent.click(screen.getByRole("checkbox", { name: "two" }));
  expect(onChange).toHaveBeenCalledWith(["shared", "two"]);
  expect(fetchModelSelectOptions).toHaveBeenCalledWith("one", expect.any(AbortSignal));
  expect(fetchModelSelectOptions).toHaveBeenCalledWith("two", expect.any(AbortSignal));
});

it("keeps candidate model guidance out of the form body and exposes it from the label help tooltip", async () => {
  render(<AgentCandidateModelsField workspaceCode="one" value={[]} onChange={vi.fn()} />);
  const help = "workspaceAgents.internal.candidateModels.help";
  const helpButton = screen.getByRole("button", { name: "workspaceAgents.internal.candidateModels.label" });
  expect(screen.queryByText(help)).toBeNull();
  expect(helpButton.getAttribute("data-slot")).toBe("tooltip-trigger");
  expect(document.querySelector(".overscroll-y-contain")).not.toBeNull();
});
