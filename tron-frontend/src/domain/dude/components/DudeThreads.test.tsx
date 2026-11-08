// @vitest-environment jsdom
import { MemoryRouter } from "react-router";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { SessionHistory } from "@personai/agent-ui-chat-core";
import { DudeThreads } from "./DudeThreads";
const api = vi.hoisted(() => ({ list: vi.fn(), history: vi.fn(), send: vi.fn(), answer: vi.fn() }));
vi.mock("../api/dudeApi", () => ({ listDudeThreads: api.list, getDudeThreadHistory: api.history, sendDudeThreadMessage: api.send, answerDudeThreadInteraction: api.answer }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/domain/auth", () => ({ useAuthSession: () => ({ session: { user: { displayName: "Human" } } }) }));
vi.mock("@/domain/chat", () => ({ useChatHistoryLabels: () => undefined, useChatWorkRailLabels: () => undefined }));
vi.mock("@personai/agent-ui-chat-react", () => ({ ChatPanel: (props: any) => <div>
  {props.messages.map((message: any) => <p key={message.id}>{message.provenance?.sender.displayName}: {message.content}</p>)}
  <input aria-label="Follow-up" value={props.draft} onChange={event => props.onDraftChange(event.target.value)} />
  <button onClick={props.onSend}>Send</button>
  {props.userInputRequest ? <button onClick={() => props.onAnswerUserInput([{ questionId: "question-a", answerKind: "Option", optionId: "option-a" }])}>Answer</button> : null}
</div> }));
const initial: SessionHistory = { sessionId: "thread-a", version: "1", nextCursor: null, runs: [], activeRuns: [], pendingInteractions: [], records: { messages: [{id:"message-a",runId:"run-a",stepId:null,role:"User",content:[{type:"Text",text:"Do work"}],structured:null,status:"Completed",createdAtMs:1,provenance:{sender:{kind:"Dude",id:"dude-a",displayName:"Dude A"}}}], steps: [], toolInvocations: [], interactions: [] } };
function mount() { render(<MemoryRouter><QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><DudeThreads dudeCode="dude-a" active requestedThread="thread-a" /></QueryClientProvider></MemoryRouter>); }
beforeEach(() => { vi.clearAllMocks(); api.list.mockResolvedValue([{ sessionCode:"thread-a",title:"Work",status:"Completed",latestRunCode:"run-a" }]); api.history.mockResolvedValue(initial); api.send.mockResolvedValue({}); api.answer.mockResolvedValue({}); });
afterEach(cleanup);
it("keeps the Dude sender and submits human follow-up without an identity override", async () => {
  mount(); await screen.findByText("Dude A: Do work");
  fireEvent.change(screen.getByLabelText("Follow-up"), {target:{value:"Please refine"}}); fireEvent.click(screen.getByText("Send"));
  await waitFor(() => expect(api.send).toHaveBeenCalledWith("dude-a","thread-a","Please refine"));
  await waitFor(() => expect((screen.getByLabelText("Follow-up") as HTMLInputElement).value).toBe(""));
});
it("retains a failed follow-up for retry", async () => {
  api.send.mockRejectedValue(new Error("Thread unavailable")); mount(); await screen.findByLabelText("Follow-up");
  fireEvent.change(screen.getByLabelText("Follow-up"), {target:{value:"Keep this"}}); fireEvent.click(screen.getByText("Send"));
  await screen.findByText("Thread unavailable"); expect((screen.getByLabelText("Follow-up") as HTMLInputElement).value).toBe("Keep this");
});
it("sends the native questionnaire answer instead of product question codes", async () => {
  api.history.mockResolvedValue({...initial,pendingInteractions:[{id:"interaction-a",runId:"run-a",invocationId:null,prompt:[{type:"Text",text:"Pick one"}],responseSchema:{type:"object"},presentation:{questions:[{id:"question-a",question:"Choose",options:[{id:"option-a",label:"A"}]}]},response:null,expiresAtMs:null}]});
  mount(); fireEvent.click(await screen.findByText("Answer"));
  await waitFor(() => expect(api.answer).toHaveBeenCalledWith("dude-a","thread-a","interaction-a",{answers:{"question-a":{option_id:"option-a"}}}));
});
