// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AgentCandidateModelsField } from "./AgentCandidateModelsField";
import { fetchWorkspaceSelectOptions } from "@/domain/model/api";

vi.mock("@/shared", () => ({ useWorkspaceSpaces: () => ({}) }));
vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/domain/admin/api", () => ({ fetchWorkspaceOptions: vi.fn(async () => [{ workspaceCode: "one" }, { workspaceCode: "two" }]) }));
vi.mock("@/domain/model/api", () => ({ fetchWorkspaceSelectOptions: vi.fn(async (workspace: string) => [
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
  expect(fetchWorkspaceSelectOptions).toHaveBeenCalledWith("one", expect.any(AbortSignal));
  expect(fetchWorkspaceSelectOptions).toHaveBeenCalledWith("two", expect.any(AbortSignal));
});
