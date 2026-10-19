// @vitest-environment jsdom
import {useState,type ReactNode} from "react";
import {renderHook,waitFor,cleanup} from "@testing-library/react";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import {afterEach,beforeEach,expect,it,vi} from "vitest";
import {useSavedContextAssembly} from "./useSavedContextAssembly";
import type {ContextAssemblySnapshot} from "../model/contextAssemblyReport";
const request=vi.hoisted(()=>vi.fn());
vi.mock("@/shared/api/httpClient",()=>({requestApi:request}));
vi.mock("@/store/authStore",()=>({useAuthStore:(select:(s:unknown)=>unknown)=>select({session:{user:{userCode:"user"}}})}));
const snapshot=(id:string):ContextAssemblySnapshot=>({blocks:[],snapshotSequence:id,inputCapacity:1000,protocolTokens:0,estimatedInputTokens:10,modelCode:"model",stage:"chat",step:1,attempt:1,budgetBreakdown:null});
function wrapper({children}:{children:ReactNode}) {const [client]=useState(()=>new QueryClient({defaultOptions:{queries:{retry:false}}}));return <QueryClientProvider client={client}>{children}</QueryClientProvider>}
beforeEach(()=>{request.mockReset();});afterEach(cleanup);
it("loads the persisted report when reopening a session",async()=>{
 const saved=snapshot("7");request.mockResolvedValue({snapshot:saved});
 const view=renderHook(()=>useSavedContextAssembly("session",null),{wrapper});
 await waitFor(()=>expect(view.result.current.snapshot).toEqual(saved));
 expect(request).toHaveBeenCalledWith("GET /api/v1/chats/sessions/get-context-assembly",expect.objectContaining({queryParams:{sessionCode:"session"}}));
});
it("uses the persisted live event report without a second request",()=>{
 const live=snapshot("8");const view=renderHook(()=>useSavedContextAssembly("session",live),{wrapper});
 expect(view.result.current.snapshot).toBe(live);expect(request).not.toHaveBeenCalled();
});
it("does not fabricate blocks for sessions without a report",async()=>{
 request.mockResolvedValue({snapshot:null});const view=renderHook(()=>useSavedContextAssembly("new-session",null),{wrapper});
 await waitFor(()=>expect(view.result.current.loading).toBe(false));
 expect(view.result.current.snapshot).toBeNull();
});
it("does not leak the previous session report when switching sessions",async()=>{
 const first=snapshot("1"),second=snapshot("2");request.mockImplementation((_url:string,options:{queryParams:{sessionCode:string}})=>Promise.resolve({snapshot:options.queryParams.sessionCode==="a"?first:second}));
 const view=renderHook(({sessionCode})=>useSavedContextAssembly(sessionCode,null),{wrapper,initialProps:{sessionCode:"a"}});
 await waitFor(()=>expect(view.result.current.snapshot).toEqual(first));
 view.rerender({sessionCode:"b"});
 expect(view.result.current.snapshot).not.toEqual(first);
 await waitFor(()=>expect(view.result.current.snapshot).toEqual(second));
});
