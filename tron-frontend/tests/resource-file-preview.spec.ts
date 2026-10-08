import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

function previewPdf() {
 const stream="BT /F1 20 Tf 40 220 Td (Resource file preview) Tj ET";
 const objects=[
  "<< /Type /Catalog /Pages 2 0 R >>",
  "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
  "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
  "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  `<< /Length ${Buffer.byteLength(stream)} >>\nstream\n${stream}\nendstream`,
 ];
 let pdf="%PDF-1.4\n";
 const offsets=[0];
 objects.forEach((object,index)=>{offsets.push(Buffer.byteLength(pdf));pdf+=`${index+1} 0 obj\n${object}\nendobj\n`;});
 const xref=Buffer.byteLength(pdf);
 pdf+=`xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
 pdf+=offsets.slice(1).map(offset=>`${String(offset).padStart(10,"0")} 00000 n \n`).join("");
 pdf+=`trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF`;
 return Buffer.from(pdf);
}

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async()=>{frontend=await startResourceUiFrontend("/knowledge-base/kb-preview",{VITE_TRON_DATA_SOURCE:"mock",VITE_TRON_RESOURCE_DATA_SOURCE:"api"});});
test.afterAll(async()=>{await stopResourceUiFrontend(frontend);});

for(const locale of ["en-US","zh-CN"] as const) {
 test(`Knowledge resources and individual files share PDF/image previews in ${locale}`,async({page},testInfo)=>{
  const zh=locale==="zh-CN";
  await routeResourceUiShellApi(page);await seedResourceUiShell(page,locale);
  await page.route("**/api/v1/chats/session-groups?*",route=>route.fulfill({json:{code:"SUCCESS",message:"ok",data:[]}}));
  const image=Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jQmcAAAAASUVORK5CYII=","base64");
  const binaries:{workspaceCode:string;resourceCode:string}[]=[];
  let replaced=false;
  await page.route("**/api/v1/resources/**",async route=>{
   const request=route.request(),path=new URL(request.url()).pathname;
   expect(request.headers()["accept-language"]).toBe(locale);
   if(path.endsWith("/upload-binary")){
    replaced=true;return route.fulfill({json:{code:"SUCCESS",message:"ok",data:{action:"UPLOADED",resourceCode:"res-pdf",fileName:"replacement.png",byteSize:image.length,targetPath:null}}});
   }
   const body=request.postDataJSON();expect(body.workspaceCode).toBe("TRON_CORE");
   if(path.endsWith("/get")) {
    const fileName=body.resourceCode==="res-image"?"table.png":body.resourceCode==="res-text"?"report.html":replaced?"replacement.png":"guide.pdf";
    return route.fulfill({json:{code:"SUCCESS",message:"ok",data:{workspaceCode:"TRON_CORE",resourceCode:body.resourceCode,kind:body.resourceCode==="res-image"?"Image":body.resourceCode==="res-text"?"File":"Pdf",name:fileName,description:"",sourceKind:"Local",allowInheritance:false,file:{fileName,fileExtension:fileName.split(".").at(-1),byteSize:100},attributes:null,createdAt:1768000000000,updatedAt:1768000000000}}});
   }
   if(path.endsWith("/download-binary")) {
    binaries.push(body);
    if(body.resourceCode==="res-text")return route.fulfill({contentType:"text/html",body:"<h1>Private report</h1>"});
    if(body.resourceCode==="res-image"||replaced)return route.fulfill({contentType:"image/png",body:image});
    return route.fulfill({contentType:"application/octet-stream",body:previewPdf()});
   }
   throw new Error(`Unexpected resource request: ${path}`);
  });
  await page.route("**/api/v1/knowledge-bases**",route=>route.fulfill({json:{code:"SUCCESS",message:"ok",data:{id:"kb-preview",name:"Guidelines",workspaceCode:"TRON_CORE",agentCodes:[],defaultChunkConfig:{mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2},documents:[{id:"doc-preview",fileName:"guide.pdf",name:"Guide",resourceCode:"res-pdf",chunkConfig:null,status:"READY",chunkCount:3,indexRevision:1,parsingArtifact:{effectiveTier:"standard",parserVersion:"4.0.10",imageResourceCodes:["res-image"]}}]}}}));
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-preview`);
  await page.getByRole("row").filter({hasText:"guide.pdf"}).getByRole("button").click();
  await page.getByRole("menuitem",{name:zh?"预览":"Preview",exact:true}).click();
  const aggregate=page.getByRole("dialog");
  await expect(aggregate.locator("iframe")).toBeVisible();await expect(aggregate.getByRole("img")).toBeVisible();
  await aggregate.locator("figure").filter({has:page.locator("iframe")}).getByRole("button").click();
  await expect(page).toHaveURL(/resource-management\/files\/detail\?resourceCode=res-pdf&preview=1/);
  const dialog=page.getByRole("dialog");
  await expect(dialog.getByRole("heading",{name:"guide.pdf"})).toBeVisible();
  await expect(dialog.locator("iframe")).toHaveAttribute("src",/^blob:/);
  const pdfUrl=await dialog.locator("iframe").getAttribute("src");
  const pdfDownloads=binaries.filter(item=>item.resourceCode==="res-pdf").length;
  const maximize=dialog.getByRole("button",{name:zh?"全屏预览":"Fullscreen preview",exact:true});
  const restore=dialog.getByRole("button",{name:zh?"退出全屏":"Exit fullscreen",exact:true});
  await maximize.click();
  await expect(restore).toBeVisible();
  const viewport=page.viewportSize()!;
  await expect.poll(async()=>Math.round((await dialog.boundingBox())!.width)).toBe(viewport.width);
  await expect.poll(async()=>Math.round((await dialog.boundingBox())!.height)).toBe(viewport.height);
  await expect(dialog.locator("iframe")).toHaveAttribute("src",pdfUrl!);
  expect(binaries.filter(item=>item.resourceCode==="res-pdf")).toHaveLength(pdfDownloads);
  await page.screenshot({animations:"disabled",path:testInfo.outputPath(`resource-preview-fullscreen-${locale}.png`)});
  await restore.click();
  await expect(maximize).toBeVisible();
  expect((await dialog.boundingBox())!.width).toBeLessThan(viewport.width);
  await maximize.click(); await restore.press("Escape");
  await expect(maximize).toBeVisible(); await expect(dialog).toBeVisible();
  await expect(dialog.locator("iframe")).toHaveAttribute("src",pdfUrl!);
  await page.screenshot({animations:"disabled",path:testInfo.outputPath(`resource-preview-${locale}.png`)});
  await dialog.getByRole("button",{name:zh?"关闭预览":"Close preview",exact:true}).click();
  await page.locator("#resource-editor-replace-input").setInputFiles({name:"replacement.png",mimeType:"image/png",buffer:image});
  await expect(page.getByText("replacement.png",{exact:true})).toBeVisible();
  await page.getByRole("button",{name:zh?"预览":"Preview",exact:true}).click();
  await expect(dialog.getByRole("img",{name:"replacement.png"})).toBeVisible();
  await expect(dialog.locator("iframe")).toHaveCount(0);
  await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-preview`);
  await page.getByRole("row").filter({hasText:"guide.pdf"}).getByRole("button").click();
  await page.getByRole("menuitem",{name:zh?"预览":"Preview",exact:true}).click();
  await aggregate.locator("figure").filter({has:page.getByRole("img")}).getByRole("button").click();
  await expect(page).toHaveURL(/resourceCode=res-image&preview=1/);
  await expect(dialog.getByRole("img",{name:"table.png"})).toBeVisible();
  const imageUrl=await dialog.getByRole("img",{name:"table.png"}).getAttribute("src");
  await maximize.click(); await expect(restore).toBeVisible();
  await expect(dialog.getByRole("img",{name:"table.png"})).toHaveAttribute("src",imageUrl!);
  const imageBox=(await dialog.getByRole("img",{name:"table.png"}).boundingBox())!;
  expect(imageBox.height).toBeGreaterThan(viewport.height*0.8);
  await restore.click(); await expect(maximize).toBeVisible();
  await page.goto(`${frontend!.baseUrl}/resource-management/files/detail?resourceCode=res-text&preview=1`);
  await expect(dialog.getByText("<h1>Private report</h1>",{exact:true})).toBeVisible();
  await expect(dialog.locator("iframe,h1")).toHaveCount(0);
  await maximize.click(); await expect(restore).toBeVisible();
  await expect(dialog.getByText("<h1>Private report</h1>",{exact:true})).toBeVisible();
  await restore.press("Escape"); await expect(maximize).toBeVisible();
  await maximize.press("Escape"); await expect(dialog).toHaveCount(0);
  expect(binaries.some(item=>item.resourceCode==="res-pdf")).toBe(true);
  expect(binaries.some(item=>item.resourceCode==="res-image")).toBe(true);
  expect(binaries.every(item=>item.workspaceCode==="TRON_CORE")).toBe(true);
 });
}
