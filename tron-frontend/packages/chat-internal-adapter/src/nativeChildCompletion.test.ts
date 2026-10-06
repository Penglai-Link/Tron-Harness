import { afterEach, describe, expect, it, vi } from "vitest";
import { createInternalChatAdapter } from "./index";
import { createOpenApiChatAdapter } from "../../chat-openapi-adapter/src/index";
import type { ChatAdapter, ChatAguiStreamEvent, ChatSession, ChatSendResult } from "@tron/chat-core";
afterEach(()=>vi.unstubAllGlobals());
const session:ChatSession={id:1,sessionCode:"SESSION",workspaceCode:"WORKSPACE",title:"Goal",status:"ACTIVE",createdAt:"2026-10-02T00:00:00Z",updatedAt:"2026-10-02T00:00:00Z"};
const factories:[string,()=>ChatAdapter][]=[["internal",()=>createInternalChatAdapter()],["openapi",()=>createOpenApiChatAdapter({openAppCode:"TEST-APP",appKey:"test-signing-only",userCode:"TEST-USER",nonceFactory:()=>"test-nonce",timestampFactory:()=>1})]];
function response(frames:Record<string,unknown>[]) {return new Response(frames.map((frame)=>"event: "+frame.type+"\ndata: "+JSON.stringify(frame)+"\n\n").join(""),{headers:{"Content-Type":"text/event-stream"}});}
const childError={type:"RUN_ERROR",runId:"CHILD-PRODUCT",threadId:"SESSION",code:"MODEL_PROVIDER_FAILED",message:"child provider failed",result:{status:"Failed",output:"partial native error"},metadata:{framework:{runId:"SDK-CHILD",executionId:"EXEC-CHILD",sourceSequence:3},tron:{eventId:"native:EXEC-CHILD:3:0"}}};
async function invoke(adapter:ChatAdapter,mode:string,events:ChatAguiStreamEvent[]) {
 const handlers={onAguiEvent:(event:ChatAguiStreamEvent)=>events.push(event)};
 return mode==="send" ? adapter.streamMessage(session,{workspaceCode:"WORKSPACE",content:"Continue the goal"},handlers) : adapter.answerUserInputRequestStream!(session,"NATIVE-APPROVAL",{answers:[],resume:[{interruptId:"NATIVE-APPROVAL",status:"resolved",payload:true}]},handlers);
}
describe.each(factories)("%s native child SSE completion",(_name,factory)=> {
 it.each(["send","resume"])("%s preserves native terminal results until product completion",async(mode)=> {
  const nativeResult={output:{Text:"native output"},content:[],structured:null,interactions:[],status:"Completed",usage:{},steps:1,tool_calls:0};
  const native={type:"RUN_FINISHED",runId:"SDK-CHILD",threadId:"SESSION",metadata:{framework:{runId:"SDK-CHILD",executionId:"EXEC-CHILD",sourceSequence:7}},outcome:{type:"success"},result:nativeResult};
  const result:ChatSendResult={session,run:{runCode:"PRODUCT-FINAL",workspaceCode:"WORKSPACE",status:"COMPLETED",finalOutput:"Product output",stepCount:1},messages:[],userInputRequest:null};
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(response([native,{type:"RUN_FINISHED",runId:"PRODUCT-FINAL",productCompletion:true,result}])));
  const events:ChatAguiStreamEvent[]=[];
  const actual=await invoke(factory(),mode,events);
  expect(actual.run.runCode).toBe("PRODUCT-FINAL");
  expect(events).toHaveLength(2);
  expect(events[0].result).toEqual(nativeResult);
 });
 it.each(["send","resume"].flatMap((mode)=>[
  {session:{sessionCode:"NATIVE-DATA"},run:{runCode:"NATIVE-DATA"},messages:[null]},
  {session:{sessionCode:"NATIVE-DATA",status:1},run:{runCode:"NATIVE-DATA"},messages:[]},
  {session:{sessionCode:"NATIVE-DATA"},run:{runCode:"NATIVE-DATA",status:1},messages:[]},
  {session:{sessionCode:"NATIVE-DATA"},run:{runCode:"NATIVE-DATA"},messages:[{messageCode:"M",sessionCode:"N",role:1}]},
  {session:{sessionCode:"NATIVE-DATA"},run:{runCode:"NATIVE-DATA"},messages:[{messageCode:"M",sessionCode:"N",attachments:1}]},
  {session:{sessionCode:"NATIVE-DATA"},run:{runCode:"NATIVE-DATA"},messages:[],userInputRequest:{questions:null}},
  {session:{sessionCode:"NATIVE-DATA"},run:{runCode:"NATIVE-DATA"},messages:[],userInputRequest:{questions:[{options:1}]}},
 ].map((opaque)=>({mode,opaque}))))("$mode preserves opaque native results with colliding product keys: $opaque",async({mode,opaque})=> {
  const metadata={framework:{runId:"SDK-CHILD",executionId:"EXEC-CHILD",sourceSequence:7}};
  const result:ChatSendResult={session,run:{runCode:"PRODUCT-FINAL",workspaceCode:"WORKSPACE",status:"COMPLETED",finalOutput:"Product output",stepCount:1},messages:[],userInputRequest:null};
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(response([
   {type:"RUN_ERROR",runId:"SDK-CHILD",threadId:"SESSION",metadata,code:"MODEL_PROVIDER_FAILED",result:opaque},
   {type:"RUN_FINISHED",runId:"SDK-CHILD",threadId:"SESSION",metadata,result:opaque},
   {type:"RUN_FINISHED",runId:"PRODUCT-FINAL",productCompletion:true,result},
  ])));
  const events:ChatAguiStreamEvent[]=[];
  const actual=await invoke(factory(),mode,events);
  expect(actual.run.runCode).toBe("PRODUCT-FINAL");
  expect(events).toHaveLength(3);
  expect(events[0].result).toEqual(opaque);
  expect(events[1].result).toEqual(opaque);
 });
 it.each(["send","resume"])("%s rejects malformed marked product completion",async(mode)=> {
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(response([{type:"RUN_FINISHED",runId:"PRODUCT",productCompletion:true,result:{status:"Completed",output:"native shape"}}])));
  await expect(invoke(factory(),mode,[])).rejects.toMatchObject({code:"CHAT_STREAM_COMPLETION_INVALID"});
 });

 it.each(["send","resume"].flatMap((mode)=>["COMPLETED","AWAITING_USER_INPUT"].map((status)=>[mode,status]))) ("%s retains child errors and returns actual final %s",async(mode,status)=> {
  const native={runId:"SDK-FINAL",status:status==="COMPLETED"?"Completed":"AwaitingInput",interactions:status==="COMPLETED"?[]:[{id:"NATIVE-APPROVAL",kind:{Approval:{invocationId:"INVOCATION"}},prompt:"Allow?",responseSchema:{type:"boolean"}}]};
  const result:ChatSendResult={session,run:{runCode:"ACTUAL-FINAL",workspaceCode:"WORKSPACE",status:status as "COMPLETED"|"AWAITING_USER_INPUT",finalOutput:"Actual output",stepCount:1,...{native}},messages:[],userInputRequest:null};
  const outcome=status==="COMPLETED"?{type:"success"}:{type:"interrupt",interrupts:[{id:"NATIVE-APPROVAL",reason:"approval_required",responseSchema:{type:"boolean"}}]};
  const frames=[{type:"RUN_STARTED",runId:"INITIAL",threadId:"SESSION"},childError,{type:"RUN_FINISHED",runId:"ACTUAL-FINAL",threadId:"SESSION",productCompletion:true,outcome,result}];
  const fetch=vi.fn().mockResolvedValue(response(frames));vi.stubGlobal("fetch",fetch);const events:ChatAguiStreamEvent[]=[];
  const actual=await invoke(factory(),mode,events);expect(actual.run.runCode).toBe("ACTUAL-FINAL");expect(actual.run.status).toBe(status);expect((actual.run as unknown as {native:unknown}).native).toEqual(native);
  expect(events.map((event)=>event.type)).toEqual(["RUN_STARTED","RUN_ERROR","RUN_FINISHED"]);expect(events[1].runId).toBe("CHILD-PRODUCT");expect(events[1].result).toEqual(childError.result);
  if(mode==="resume") expect(JSON.parse(fetch.mock.calls[0][1].body).resume).toEqual([{interruptId:"NATIVE-APPROVAL",status:"resolved",payload:true}]);
 });
 it.each(["send","resume"])("%s rejects actual product failure after preserving the child frame",async(mode)=> {
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(response([childError,{type:"RUN_ERROR",runId:"INITIAL",productCompletion:true,code:"RUN_FAILED",message:"Actual execution failed"}])));
  const events:ChatAguiStreamEvent[]=[];await expect(invoke(factory(),mode,events)).rejects.toMatchObject({code:"RUN_FAILED"});expect(events[0].runId).toBe("CHILD-PRODUCT");
 });
 it.each(["send","resume"])("%s reports typed product ack loss while preserving authoritative native success",async(mode)=> {
  vi.stubGlobal("fetch",vi.fn().mockResolvedValue(response([{type:"RUN_FINISHED",runId:"INITIAL",productCompletion:true,outcome:{type:"success"},presentationIncomplete:true,productCompletionError:{code:"CHAT_STREAM_COMPLETION_UNAVAILABLE",message:"Product completion unavailable"}}])));
  const events:ChatAguiStreamEvent[]=[];await expect(invoke(factory(),mode,events)).rejects.toMatchObject({code:"CHAT_STREAM_COMPLETION_UNAVAILABLE"});expect(events[0].outcome).toEqual({type:"success"});
 });
});
