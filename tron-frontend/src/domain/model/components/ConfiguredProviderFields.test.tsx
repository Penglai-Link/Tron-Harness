// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useState } from "react";
import { ConfiguredProviderFields } from "./ConfiguredProviderFields";
import { emptyProviderDraft, providerDraftToPayload, type ConfiguredProviderDraft } from "./configuredProviderForm";

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
afterEach(cleanup);

it("displays the saved summary, clears it on focus and never submits it as a key", () => {
  let current: ConfiguredProviderDraft = { ...emptyProviderDraft(), hasApiKey: true, apiKeySummary: "sk-123***sdf-hsd" };
  function Controlled() {
    const [draft, setDraft] = useState(current);
    current = draft;
    return <ConfiguredProviderFields draft={draft} disabled={false} onChange={setDraft} />;
  }
  render(<Controlled />);
  const input = screen.getByLabelText("configuredProviders.fields.apiKey", { selector: "input" }) as HTMLInputElement;
  expect(input.value).toBe("sk-123***sdf-hsd");
  expect(input.type).toBe("text");
  expect(providerDraftToPayload(current).apiKey).toBeNull();
  fireEvent.focus(input);
  expect(input.value).toBe("");
  expect(input.type).toBe("password");
  fireEvent.blur(input);
  expect(input.value).toBe("sk-123***sdf-hsd");
  expect(providerDraftToPayload(current).apiKey).toBeNull();
  expect(providerDraftToPayload(current)).not.toHaveProperty("apiKeySummary");
});

it("keeps a newly entered key as a separate password draft across focus changes", () => {
  let current: ConfiguredProviderDraft = { ...emptyProviderDraft(), hasApiKey: true, apiKeySummary: "sk-123***sdf-hsd" };
  function Controlled() {
    const [draft, setDraft] = useState(current);
    current = draft;
    return <ConfiguredProviderFields draft={draft} disabled={false} onChange={setDraft} />;
  }
  render(<Controlled />);
  const input = screen.getByLabelText("configuredProviders.fields.apiKey", { selector: "input" }) as HTMLInputElement;
  fireEvent.focus(input);
  fireEvent.change(input, { target: { value: "new-secret-key" } });
  fireEvent.blur(input);
  expect(input.type).toBe("password");
  fireEvent.focus(input);
  expect(input.value).toBe("new-secret-key");
  expect(providerDraftToPayload(current).apiKey).toBe("new-secret-key");
});

it("shows connection instructions only after hovering their help buttons", async () => {
  render(<ConfiguredProviderFields draft={emptyProviderDraft()} disabled={false} onChange={() => {}} />);
  expect(screen.queryByLabelText("configuredProviders.fields.providerId")).toBeNull();
  expect(screen.queryByText("configuredProviders.hints.baseUrl")).toBeNull();
  expect(screen.queryByText("configuredProviders.hints.keyOptional")).toBeNull();
  const help = screen.getByRole("button", { name: "configuredProviders.fields.baseUrl" });
  fireEvent.mouseEnter(help);
  fireEvent.mouseMove(help);
  expect(await screen.findByRole("tooltip")).toHaveProperty("textContent", "configuredProviders.hints.baseUrl");
});
