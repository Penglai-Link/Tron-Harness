import { test,expect } from "@playwright/test";
import { routeResourceUiShellApi,seedResourceUiShell,startResourceUiFrontend,stopResourceUiFrontend,type ResourceUiFrontend } from "./support/resource-ui-harness";
let frontend:ResourceUiFrontend|undefined;
test.beforeAll(async()=>{frontend=await startResourceUiFrontend("/knowledge-base/kb-synthetic",{VITE_TRON_DATA_SOURCE:"mock"});});
test.afterAll(async()=>{await stopResourceUiFrontend(frontend);});
for(const locale of ["en-US","zh-CN"] as const){
 test(`Standard inheritance and document override render in ${locale}`,async({page},testInfo)=>{
  await routeResourceUiShellApi(page);await seedResourceUiShell(page,locale);
  await page.route("**/api/v1/chats/session-groups?*",route=>route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({code:"SUCCESS",message:"ok",data:[]})}));
  const writes:unknown[]=[];
  await page.route("**/api/v1/knowledge-bases/**",async(route)=>{
   const url=new URL(route.request().url());let data:unknown;
      if (new URL(route.request().url()).pathname.endsWith("/chunking-strategies")) return route.fulfill({ json: { code: "SUCCESS", data: { strategies: ["auto", "structure", "heading", "fixed"].map(id => ({ id, version: "1" })) } } });
   if(url.pathname.endsWith("parsing-capabilities")){data={defaultTier:"standard",availableTiers:["flash","standard","advanced"],parserVersion:"4.0.10",officeTier:"flash"};}
   else if(route.request().method()==="PATCH"){writes.push(route.request().postDataJSON());data={};}
   else {data={id:"kb-synthetic",name:"Synthetic only",description:"",workspaceCode:"TRON_CORE",documentCount:1,agentCodes:[],defaultChunkConfig:{mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2},documents:[{id:"doc-synthetic",name:"Synthetic",fileName:"synthetic-scanned.pdf",resourceCode:"KB_SYNTHETIC",resourceRevision:1,chunkConfig:null,parsingTier:null,status:"UPLOADED",error:null,chunkCount:0,indexRevision:0}]};}
   await route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({code:"SUCCESS",message:"ok",data})});
  });
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-synthetic`);await expect(page.getByText("synthetic-scanned.pdf",{exact:true})).toBeVisible();
  const help=page.getByRole("button",{name:locale==="en-US"?"Document upload and deletion help":"文档上传与删除说明"});
  await expect(page.getByText(locale==="en-US"?"Supports Markdown":"支持 Markdown",{exact:false})).not.toBeVisible();
  await help.hover();await expect(page.getByRole("tooltip")).toContainText(locale==="en-US"?"MinerU formats":"MinerU 格式");
  await page.getByRole("button",{name:locale==="en-US"?"Actions for synthetic-scanned.pdf":"操作 synthetic-scanned.pdf",exact:true}).click();
  await page.getByRole("menuitem",{name:locale==="en-US"?"Settings":"设置",exact:true}).click();
  const dialog=page.getByRole("dialog");
  await expect(dialog.getByText(locale==="en-US"?"Each document can use":"每个文档可独立配置",{exact:false})).not.toBeVisible();
  await expect(dialog.getByText(locale==="en-US"?"Unavailable quality fails":"选定质量不可用时",{exact:false})).not.toBeVisible();
  await page.getByRole("button",{name:locale==="en-US"?"Parsing quality options":"解析质量选项说明"}).hover();
  const qualityHelp=page.getByRole("tooltip");
  for(const name of locale==="en-US"?["document parsing","scanned-document OCR","precise parsing"]:["文档解析","扫描件识别","高精度解析"]) await expect(qualityHelp).toContainText(name);
  await page.screenshot({animations:"disabled",path:testInfo.outputPath(`parsing-quality-help-${locale}.png`)});

  await expect(page.getByText(locale==="en-US"?"Automatic (recommended)":"自动选择（推荐）",{exact:true})).toBeVisible();
  const quality=page.getByRole("combobox").first();await quality.click();
  await page.getByRole("option",{name:locale==="en-US"?"Precise parsing":"高精度解析",exact:true}).click();
  await page.getByRole("button",{name:locale==="en-US"?"Save":"保存",exact:true}).click();
  await expect.poll(()=>writes.length).toBe(1);expect(writes[0]).toMatchObject({chunkConfig:null,parsingTier:"advanced",ocrMode:"Text"});
  await page.screenshot({animations:"disabled",path:testInfo.outputPath(`mineru-kb-ui-${locale}.png`)});
 });
 test(`Durable stages, cancellation and retry render in ${locale}`,async({page},testInfo)=>{
  await routeResourceUiShellApi(page);await seedResourceUiShell(page,locale);
  await page.route("**/api/v1/chats/session-groups?*",route=>route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({code:"SUCCESS",message:"ok",data:[]})}));
  let state="QUEUED";const writes:string[]=[];
  await page.route("**/api/v1/knowledge-bases/**",async(route)=>{
   const url=new URL(route.request().url());
   if(route.request().method()==="POST"){
    expect(route.request().postDataJSON()).toEqual({});writes.push(url.pathname);
    if(url.pathname.endsWith("/cancel"))state="CANCELLED";
    if(url.pathname.endsWith("/retry"))state="EMBEDDING";
   }
   const document={id:"doc-synthetic",name:"Synthetic",fileName:"synthetic.pdf",resourceCode:"KB_SYNTHETIC",resourceRevision:1,chunkConfig:null,status:state,stage:state==="EMBEDDING"?"Embedding":state==="PUBLISHING"?"Publishing":"Parsing",taskId:"ingest-synthetic",error:null,chunkCount:0,indexRevision:0};
   const data=route.request().method()==="POST"?document:{id:"kb-synthetic",name:"Synthetic only",description:"",workspaceCode:"TRON_CORE",documentCount:1,agentCodes:[],defaultChunkConfig:{mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2},documents:[document]};
   await route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({code:"SUCCESS",message:"ok",data})});
  });
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-synthetic`);
  const statusCell=page.locator("tbody tr").first().locator("td").nth(4);
  await expect(statusCell.getByText(locale==="en-US"?"Waiting for document parsing":"等待文档解析",{exact:true})).toBeVisible();
  const actions=page.getByRole("button",{name:locale==="en-US"?"Actions for synthetic.pdf":"操作 synthetic.pdf",exact:true});
  await actions.click();
  await expect(page.getByRole("menuitem",{name:locale==="en-US"?"Settings":"设置",exact:true})).toHaveAttribute("aria-disabled","true");
  const settings = page.getByRole("menuitem",{name:locale==="en-US"?"Settings":"设置",exact:true});
  await settings.locator("..").hover();
  await expect(page.getByRole("tooltip")).toContainText(locale==="en-US"?"while the document is processing":"文档正在处理中");
  await expect(page.getByRole("menuitem").last()).toHaveText(locale==="en-US"?"Cancel":"取消");
  await expect(page.getByRole("menuitem",{name:locale==="en-US"?"Download":"下载",exact:true})).toBeVisible();
  await expect(page.getByRole("menuitem",{name:locale==="en-US"?"Preview":"预览",exact:true})).toBeVisible();
  await expect(page.getByRole("menuitem",{name:locale==="en-US"?"Delete":"删除",exact:true})).toBeVisible();
  await page.getByRole("menuitem",{name:locale==="en-US"?"Cancel":"取消",exact:true}).click();
  await expect(statusCell.getByText(locale==="en-US"?"Cancelled":"已取消",{exact:true})).toBeVisible();
  await actions.click();
  await page.getByRole("menuitem",{name:locale==="en-US"?"Retry":"重试",exact:true}).click();
  await expect(statusCell.getByText(locale==="en-US"?"Generating vectors":"向量化中",{exact:true})).toBeVisible();
  await expect(statusCell.locator("p")).toHaveCount(1);
  await expect(statusCell.locator('[data-slot="badge"]')).toHaveCount(0);
  expect(writes.map(path=>path.split("/").at(-1))).toEqual(["cancel","retry"]);
  state="PUBLISHING";
  await expect(statusCell.getByText(locale==="en-US"?"Publishing index":"索引发布中",{exact:true})).toBeVisible({timeout:6000});
  await page.screenshot({animations:"disabled",path:testInfo.outputPath(`ingestion-${locale}.png`)});
 });
}

for (const locale of ["en-US", "zh-CN"] as const) {
 test(`Published chunks browse past the preview limit in ${locale}`, async ({ page }, testInfo) => {
  await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
  await page.route("**/api/v1/chats/session-groups?*", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
  const reads: number[] = [];
  await page.route("**/api/v1/knowledge-bases/**", async route => {
   const url = new URL(route.request().url());
   expect(route.request().method()).toBe("GET");
   let data: unknown;
   if (url.pathname.endsWith("/chunks")) {
    expect(url.searchParams.has("workspaceCode")).toBe(false);
    const current = Number(url.searchParams.get("page")); const size = Number(url.searchParams.get("pageSize")); reads.push(current);
    data = { total: 31, page: current, pageSize: size, items: Array.from({length: Math.min(size, 31-(current-1)*size)}, (_, n) => {
     const ordinal = (current-1)*size+n;
     return { textUnitCode: `unit-${ordinal}`, ordinal, text: `Published evidence ${ordinal+1}`, headingPath: [], metadata: { location: { page: ordinal+1, charStart: 0, charEnd: 100 } } };
    }) };
   } else data = { id: "kb-synthetic", name: "Published", description: "", workspaceCode: "TRON_CORE", documentCount: 1, agentCodes: [], defaultChunkConfig: {mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2}, documents: [{id:"doc-published",name:"Published",fileName:"published.pdf",resourceCode:"KB_SYNTHETIC",resourceRevision:1,chunkConfig:null,status:"READY",error:null,chunkCount:31,indexRevision:1}] };
   await route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
  });
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-synthetic`);
  await page.getByRole("button", {name: locale === "en-US" ? "Actions for published.pdf" : "操作 published.pdf", exact: true}).click();
  await page.getByRole("menuitem", {name:locale === "en-US" ? "View chunks" : "查看分块", exact:true}).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText("Published evidence 1", {exact:true})).toBeVisible();
  const next = dialog.getByRole("button", {name:locale === "en-US" ? "Next page" : "下一页", exact:true});
  await next.click(); await expect(dialog.getByText("Published evidence 21", {exact:true})).toBeVisible();
  await next.click(); await expect(dialog.getByText("Published evidence 31", {exact:true})).toBeVisible();
  await expect(next).toBeDisabled();
  expect([...new Set(reads)]).toEqual([1,2,3]);
  await page.screenshot({animations:"disabled",path:testInfo.outputPath(`published-chunks-${locale}.png`)});
 });
}
