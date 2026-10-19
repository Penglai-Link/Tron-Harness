// @vitest-environment jsdom
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { KnowledgeQualityChat } from "./KnowledgeQualityChat";

const fixture=vi.hoisted(()=>({adapter:vi.fn((_options: unknown)=>({})),session:vi.fn(),props:null as Record<string,unknown>|null}));
vi.mock("react-i18next",()=>({useTranslation:()=>({t:(key:string)=>key})}));
vi.mock("@/i18n",()=>({applyCurrentLocaleHeader:(headers:Headers)=>{headers.set("Accept-Language",document.documentElement.lang);return headers;}}));
vi.mock("@/shared/api/httpClient",()=>({getApiAccessToken:vi.fn(()=>"test-token")}));
vi.mock("@/domain/chat",()=>({useChatHistoryLabels:()=>({}),useChatWorkRailLabels:()=>({})}));
vi.mock("@tron/chat-internal-adapter",()=>({createInternalChatAdapter:fixture.adapter}));
vi.mock("@/domain/chat/components/AgentChatWindow",()=>({AgentChatWindow:(props:Record<string,unknown>)=>{fixture.props=props;return <div>Bound chat</div>;}}));
vi.mock("../api/knowledgeQualityApi",()=>({resolveQualitySession:fixture.session}));
vi.mock("../hooks/useKnowledgeLibraries",()=>({useKnowledgeUser:()=>"owner"}));
afterEach(()=>{cleanup();vi.clearAllMocks();});
it("carries the verified library namespace to chat and suggestion requests with current locale",async()=>{
  fixture.session.mockResolvedValue({id:"9007199254740999",sessionCode:"session",workspaceCode:"PLATFORM",agentCode:"main-agent",title:"Quality",status:"ACTIVE",createdAt:"2026-10-17T00:00:00Z",updatedAt:"2026-10-17T00:00:00Z"});
  document.documentElement.lang="zh-CN";
  const client=new QueryClient({defaultOptions:{queries:{retry:false}}});
  render(<QueryClientProvider client={client}><KnowledgeQualityChat libraryId="kb-owned"/></QueryClientProvider>);
  await waitFor(()=>expect(fixture.adapter).toHaveBeenCalled());
  const options=fixture.adapter.mock.calls[0][0] as unknown as {workspaceCode:string;getHeaders:()=>Headers};
  expect(options.workspaceCode).toBe("PLATFORM");expect(options.getHeaders().get("X-Knowledge-Base-Code")).toBe("kb-owned");
  expect(options.getHeaders().get("Accept-Language")).toBe("zh-CN");
  document.documentElement.lang="en-US";expect(options.getHeaders().get("Accept-Language")).toBe("en-US");
  expect(fixture.props?.knowledgeBaseId).toBe("kb-owned");
  expect(fixture.props?.config).toMatchObject({workspaceCode:"PLATFORM",showNewSession:false,showSessionList:false});
});
