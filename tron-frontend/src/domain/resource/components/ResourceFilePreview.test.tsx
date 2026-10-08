// @vitest-environment jsdom
import { Blob as NodeBlob } from "node:buffer";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ResourceFilePreview } from "./ResourceFilePreview";

const fixture=vi.hoisted(()=>({user:"user-a",download:vi.fn(),error:vi.fn()}));
vi.mock("../api/resourceApi",()=>({downloadResourceBinary:fixture.download}));
vi.mock("@/store/authStore",()=>({useAuthStore:(selector:(state:unknown)=>unknown)=>selector({session:{user:{userCode:fixture.user}}})}));
vi.mock("@/components/ui/toast",()=>({useToast:()=>fixture}));
vi.mock("react-i18next",()=>({useTranslation:()=>({t:(key:string)=>key})}));
const create=vi.fn(); const revoke=vi.fn();
function response(content="PDF",mime="application/octet-stream") {
 return {blob:new NodeBlob([content],{type:mime}),contentDisposition:null};
}
function mount(workspace="space-a",fileName="guide.pdf",kind:"PDF"|"IMAGE"|"FILE"="PDF",format?:"image") {
 const client=new QueryClient({defaultOptions:{queries:{retry:false}}});
 const element=(scope:string)=> <QueryClientProvider client={client}><ResourceFilePreview workspaceCode={scope} resourceCode="res-a" fileName={fileName} kind={kind} format={format} /></QueryClientProvider>;
 const view=render(element(workspace));
 return {...view,refresh:(scope=workspace)=>view.rerender(element(scope))};
}
beforeEach(()=>{
 vi.clearAllMocks();fixture.user="user-a";fixture.download.mockResolvedValue(response());
 create.mockImplementation(()=>`blob:preview-${create.mock.calls.length}`);
 vi.stubGlobal("URL",Object.assign(URL,{createObjectURL:create,revokeObjectURL:revoke}));
});
afterEach(()=>{cleanup();vi.unstubAllGlobals();});
describe("resource file preview",()=>{
 it("normalizes private PDF bytes for the native viewer and revokes its URL on close",async()=>{
  const view=mount();
  await waitFor(()=>expect(screen.getByTitle("guide.pdf").getAttribute("src")).toBe("blob:preview-1"));
  expect(create.mock.calls[0][0].type).toBe("application/pdf");
  expect(fixture.download).toHaveBeenCalledWith({workspaceCode:"space-a",resourceCode:"res-a"},expect.any(AbortSignal));
  view.unmount();expect(revoke).toHaveBeenCalledWith("blob:preview-1");
 });
 it("renders knowledge images as images even when their accessible label includes the PDF filename",async()=>{
  fixture.download.mockResolvedValue(response("image","image/png"));
  mount("space-a","Image 1 in guide.pdf","IMAGE","image");
  expect(await screen.findByRole("img",{name:"Image 1 in guide.pdf"})).toBeTruthy();
  expect(document.querySelector("iframe")).toBeNull();
 });
 it.each(["workspace","user"] as const)("removes old file content while loading a new %s scope",async(scope)=>{
  const view=mount();await waitFor(()=>expect(screen.getByTitle("guide.pdf")).toBeTruthy());
  const oldUrl=screen.getByTitle("guide.pdf").getAttribute("src");
  let complete!:(value:ReturnType<typeof response>)=>void;
  fixture.download.mockImplementationOnce(()=>new Promise(resolve=>{complete=resolve;}));
  if(scope==="user")fixture.user="user-b";
  view.refresh(scope==="workspace"?"space-b":"space-a");
  expect(screen.queryByTitle("guide.pdf")).toBeNull();
  expect(revoke).toHaveBeenCalledWith(oldUrl);
  await waitFor(()=>expect(fixture.download).toHaveBeenCalledTimes(2));
  complete(response("new"));
  await waitFor(()=>expect(screen.getByTitle("guide.pdf").getAttribute("src")).not.toBe(oldUrl));
 });
 it("shows a request failure through toast and retries without downloading through an unauthenticated URL",async()=>{
  fixture.download.mockRejectedValueOnce(new Error("Forbidden"));mount();
  await waitFor(()=>expect(fixture.error).toHaveBeenCalledWith("Forbidden"));
  fireEvent.click(screen.getByRole("button",{name:"resourceEditor.preview.retry"}));
  await waitFor(()=>expect(screen.getByTitle("guide.pdf")).toBeTruthy());
  expect(fixture.download).toHaveBeenCalledTimes(2);
 });
 it("shows HTML as text without executing or embedding its markup",async()=>{
  fixture.download.mockResolvedValue(response('<script>window.unsafe=true</script><h1>Report</h1>',"text/html"));
  mount("space-a","report.html","FILE");
  await screen.findByText('<script>window.unsafe=true</script><h1>Report</h1>');
  expect(document.querySelector("iframe,script,h1")).toBeNull();expect(create).not.toHaveBeenCalled();
 });
 it("does not embed an unsupported binary format",async()=>{
  mount("space-a","archive.zip","FILE");
  await screen.findByText("resourceEditor.preview.unsupported");
  expect(document.querySelector("iframe,img")).toBeNull();expect(create).not.toHaveBeenCalled();
 });
});
