// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AgentCandidateModelsField } from "./AgentCandidateModelsField";
import { fetchModelSelectOptions } from "@/domain/model/api";

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/domain/model/api", () => ({ fetchModelSelectOptions: vi.fn(async () => [
  { optionCode: "shared", modelCode: "shared", displayName: "Shared text", modelType: "TEXT", enabled: true, contextWindowTokens: 32000 },
  { optionCode: "other", modelCode: "other", displayName: "Other text", modelType: "TEXT", enabled: true, contextWindowTokens: 32000 },
]) }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });

it.each(["$platform", ""])("loads the platform catalog once for scope '%s' and preserves selected candidates", async (workspaceCode) => {
  const onChange = vi.fn();
  render(<AgentCandidateModelsField workspaceCode={workspaceCode} value={["shared"]} onChange={onChange} />);
  expect(await screen.findByRole("checkbox", { name: "Shared text" })).toBeDefined();
  expect(screen.getAllByRole("checkbox", { name: "Shared text" })).toHaveLength(1);
  expect((screen.getByRole("checkbox", { name: "Shared text" }) as HTMLInputElement).checked).toBe(true);
  fireEvent.click(screen.getByRole("checkbox", { name: "Other text" }));
  expect(onChange).toHaveBeenCalledWith(["shared", "other"]);
  expect(fetchModelSelectOptions).toHaveBeenCalledTimes(1);
  expect(fetchModelSelectOptions).toHaveBeenCalledWith("", expect.any(AbortSignal));
});

it("keeps candidate model guidance out of the form body and exposes it from the label help tooltip", async () => {
  render(<AgentCandidateModelsField workspaceCode="one" value={[]} onChange={vi.fn()} />);
  await screen.findByRole("checkbox", { name: "Shared text" });
  expect(fetchModelSelectOptions).toHaveBeenCalledTimes(1);
  expect(fetchModelSelectOptions).toHaveBeenCalledWith("one", expect.any(AbortSignal));
  const help = "workspaceAgents.internal.candidateModels.help";
  const helpButton = screen.getByRole("button", { name: "workspaceAgents.internal.candidateModels.label" });
  expect(screen.queryByText(help)).toBeNull();
  expect(helpButton.getAttribute("data-slot")).toBe("tooltip-trigger");
  expect(document.querySelector(".overscroll-y-contain")).not.toBeNull();
});
