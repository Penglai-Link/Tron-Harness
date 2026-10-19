// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { pages } from "@/i18n/locales/en-US/pages";
import type { QualityRun } from "../api/knowledgeQualityApi";
import { KnowledgeQualityPanel } from "./KnowledgeQualityPanel";

const fixture=vi.hoisted(()=>({runs:vi.fn(),dataset:vi.fn(),start:vi.fn(),cancel:vi.fn()}));
vi.mock("@/shared/api/httpClient",()=>({requestApi:vi.fn()}));
vi.mock("react-i18next",()=>({useTranslation:()=>({t:(key:string,args?:Record<string,unknown>)=>String(key.split(".").reduce((v:unknown,k)=>(v as Record<string,unknown>)?.[k],pages) ?? args?.defaultValue ?? key).replace(/{{(\w+)}}/g,(_,k)=>String(args?.[k]??"")),i18n:{resolvedLanguage:"en-US"}})}));
vi.mock("../hooks/useKnowledgeLibraries",()=>({useKnowledgeUser:()=>"owner"}));
vi.mock("./KnowledgeQualityChat",()=>({KnowledgeQualityChat:()=> <p>Bound knowledge conversation</p>}));
vi.mock("../api/knowledgeQualityApi",async()=>({...await vi.importActual("../api/knowledgeQualityApi"),listQualityRuns:fixture.runs,getQualityDataset:fixture.dataset,startQualityRun:fixture.start,cancelQualityRun:fixture.cancel}));
function mount(){const onPublished=vi.fn(async()=>{});const client=new QueryClient({defaultOptions:{queries:{retry:false},mutations:{retry:false}}});render(<QueryClientProvider client={client}><KnowledgeQualityPanel libraryId="kb" name="Guide" onClose={vi.fn()} onPublished={onPublished}/></QueryClientProvider>);return{client,onPublished};}
beforeEach(()=>{vi.clearAllMocks();fixture.runs.mockResolvedValue({runs:[]});fixture.dataset.mockResolvedValue({dataset:null});});
afterEach(cleanup);
it("shows unavailable metrics as dashes and submits bounded defaults",async()=>{
  fixture.runs.mockResolvedValue({runs:[{id:"run",status:"Failed",stage:"Completed",trials:[],selectedTrial:null,baselineTraining:null,baselineHoldout:null,published:false,targetReached:false,missing:[],warnings:[],error:"provider unavailable",modelOperations:1,estimatedInputTokens:50,createdAt:0,updatedAt:0} as unknown as QualityRun]});
  fixture.start.mockRejectedValue(new Error("submission uncertain"));
  mount();await screen.findByText("provider unavailable");
  const baseline=screen.getByRole("row",{name:/Current index/});expect(within(baseline).getAllByText("—")).toHaveLength(4);expect(screen.queryByText("0%",{exact:true})).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Evaluate and optimize"}));await screen.findByText("submission uncertain");
  const id=fixture.start.mock.calls[0][1];fireEvent.click(screen.getByRole("button",{name:"Evaluate and optimize"}));
  await waitFor(()=>expect(fixture.start).toHaveBeenCalledTimes(2));expect(fixture.start.mock.calls[1][1]).toBe(id);
  expect(fixture.start.mock.calls[0][2]).toMatchObject({maxTrials:3,budget:{maxModelOperations:160,maxInputTokens:250000,maxDurationSeconds:900}});
});
it("distinguishes a published improvement below target from reaching the target",async()=>{
  const metrics={cases:4,recall:.75,mrr:.75,precision:.5,discountedRecall:.75,hitRate:.75,humanLabeledCases:0,generatedCases:4};
  fixture.dataset.mockResolvedValue({dataset:{cases:[{id:"missing",query:"Which exceptions apply?",provenance:"Generated"}]}});
  fixture.runs.mockResolvedValue({runs:[{id:"published",status:"Published",stage:"Completed",trials:[{strategy:{id:"auto",version:"1"},chunkConfig:{maxChars:1200},training:metrics,holdout:metrics,chunkCount:8,error:null}],selectedTrial:0,baselineTraining:{...metrics,recall:.25},baselineHoldout:{...metrics,recall:.25},published:true,targetReached:false,missing:[{caseId:"missing"}],warnings:[],error:null,modelOperations:10,estimatedInputTokens:100,createdAt:0,updatedAt:0} as unknown as QualityRun]});
  const {onPublished}=mount();await screen.findByText("The improved strategy is published. Old indexes are removed.");
  expect(screen.getByText("Results improved but have not yet reached the target.")).toBeTruthy();
  expect(await screen.findByText(/this is a source self-test/)).toBeTruthy();
  expect(screen.getByText("Which exceptions apply?")).toBeTruthy();await waitFor(()=>expect(onPublished).toHaveBeenCalledTimes(1));
});
