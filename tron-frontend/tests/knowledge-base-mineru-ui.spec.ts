import { test,expect } from "@playwright/test";
import { routeResourceUiShellApi,seedResourceUiShell,startResourceUiFrontend,stopResourceUiFrontend,type ResourceUiFrontend } from "./support/resource-ui-harness";
let frontend:ResourceUiFrontend|undefined;
test.beforeAll(async()=>{frontend=await startResourceUiFrontend("/knowledge-base/kb-synthetic",{VITE_TRON_DATA_SOURCE:"mock"});});
test.afterAll(async()=>{await stopResourceUiFrontend(frontend);});
for(const locale of ["en-US","zh-CN"] as const){
 test(`Standard inheritance and document override render in ${locale}`,async({page},testInfo)=>{
  await routeResourceUiShellApi(page);await seedResourceUiShell(page,locale);
  const writes:unknown[]=[];
  await page.route("**/api/v1/knowledge-bases/**",async(route)=>{
   const url=new URL(route.request().url());let data:unknown;
   if(url.pathname.endsWith("parsing-capabilities")){data={defaultTier:"standard",availableTiers:["flash","standard","advanced"],parserVersion:"4.0.10",officeTier:"flash"};}
   else if(route.request().method()==="PATCH"){writes.push(route.request().postDataJSON());data={};}
   else {data={id:"kb-synthetic",name:"Synthetic only",description:"",workspaceCode:"TRON_CORE",documentCount:1,agentCodes:[],defaultChunkConfig:{mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2},documents:[{id:"doc-synthetic",name:"Synthetic",fileName:"synthetic-scanned.pdf",resourceCode:"KB_SYNTHETIC",resourceRevision:1,chunkConfig:null,parsingTier:null,status:"UPLOADED",error:null,chunkCount:0,indexRevision:0}]};}
   await route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({code:"SUCCESS",message:"ok",data})});
  });
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-synthetic`);await expect(page.getByText("synthetic-scanned.pdf",{exact:true})).toBeVisible();
  await page.getByRole("button",{name:locale==="en-US"?"Settings":"设置",exact:true}).click();
  await expect(page.getByText(locale==="en-US"?"Use platform default (Standard)":"继承平台（标准）",{exact:true})).toBeVisible();
  const quality=page.getByRole("combobox").first();await quality.click();
  await page.getByRole("option",{name:locale==="en-US"?"Advanced":"高级",exact:true}).click();
  await page.getByRole("button",{name:locale==="en-US"?"Save":"保存",exact:true}).click();
  await expect.poll(()=>writes.length).toBe(1);expect(writes[0]).toMatchObject({workspaceCode:"TRON_CORE",chunkConfig:null,parsingTier:"advanced"});
  await page.screenshot({path:testInfo.outputPath(`mineru-kb-ui-${locale}.png`)});
 });
 test(`Durable stages, cancellation and retry render in ${locale}`,async({page},testInfo)=>{
  await routeResourceUiShellApi(page);await seedResourceUiShell(page,locale);
  let state="QUEUED";const writes:string[]=[];
  await page.route("**/api/v1/knowledge-bases/**",async(route)=>{
   const url=new URL(route.request().url());
   if(route.request().method()==="POST"){
    expect(route.request().postDataJSON()).toEqual({workspaceCode:"TRON_CORE"});writes.push(url.pathname);
    if(url.pathname.endsWith("/cancel"))state="CANCELLED";
    if(url.pathname.endsWith("/retry"))state="EMBEDDING";
   }
   const document={id:"doc-synthetic",name:"Synthetic",fileName:"synthetic.pdf",resourceCode:"KB_SYNTHETIC",resourceRevision:1,chunkConfig:null,status:state,stage:state==="EMBEDDING"?"Embedding":state==="PUBLISHING"?"Publishing":"Parsing",taskId:"ingest-synthetic",error:null,chunkCount:0,indexRevision:0};
   const data=route.request().method()==="POST"?document:{id:"kb-synthetic",name:"Synthetic only",description:"",workspaceCode:"TRON_CORE",documentCount:1,agentCodes:[],defaultChunkConfig:{mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2},documents:[document]};
   await route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({code:"SUCCESS",message:"ok",data})});
  });
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-synthetic`);
  await expect(page.locator('[data-slot="badge"]',{hasText:locale==="en-US"?"Queued":"排队中"})).toBeVisible();
  await expect(page.getByRole("button",{name:locale==="en-US"?"Settings":"设置",exact:true})).toBeDisabled();
  await page.getByRole("button",{name:locale==="en-US"?"Cancel":"取消",exact:true}).click();
  await expect(page.locator('[data-slot="badge"]',{hasText:locale==="en-US"?"Cancelled":"已取消"})).toBeVisible();
  await page.getByRole("button",{name:locale==="en-US"?"Retry":"重试",exact:true}).click();
  await expect(page.locator('[data-slot="badge"]',{hasText:locale==="en-US"?"Embedding":"向量化中"})).toBeVisible();
  expect(writes.map(path=>path.split("/").at(-1))).toEqual(["cancel","retry"]);
  state="PUBLISHING";
  await expect(page.locator('[data-slot="badge"]',{hasText:locale==="en-US"?"Publishing":"发布中"})).toBeVisible({timeout:6000});
  await page.screenshot({path:testInfo.outputPath(`ingestion-${locale}.png`)});
 });
}
