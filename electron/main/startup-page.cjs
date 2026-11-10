const escape = (value) =>
  String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

function startupPageHtml({ message, dataDir, logPath, locale = "en", theme = "matrix-dark" }) {
  const zh = locale.startsWith("zh");
  return `<!doctype html>
<html lang="${zh ? "zh-CN" : "en"}" data-theme="${theme === "light" ? "light" : "matrix-dark"}">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Tron</title>
<style>

:root { color-scheme: dark; font-family: Inter, -apple-system, BlinkMacSystemFont, sans-serif; background:#020305; color:#f3f1eb; }
* { box-sizing:border-box; }
html,body { height:100%; margin:0; }
body { display:flex; align-items:center; padding:40px clamp(24px,8vw,150px); overflow:hidden; }
.status { position:relative; width:min(360px,100%); padding:24px 0; animation:appear 600ms both; }
.brand { margin:0 0 32px; font-size:13px; font-weight:650; letter-spacing:.32em; color:#f3f1eb; }
.status-row { display:flex; align-items:center; gap:10px; font-size:14px; font-weight:450; }
.dot { width:5px; height:5px; border-radius:50%; background:#e7dbc4; box-shadow:0 0 12px #ddd0ae80; animation:pulse 2s infinite; }
.message { margin-top:14px; color:#8f98a7; font-size:12px; line-height:1.7; max-height:120px; overflow:auto; overflow-wrap:anywhere; }
.progress { height:1px; margin:28px 0; overflow:hidden; background:#ffffff15; }
.progress:after { content:""; display:block; width:35%; height:100%; background:linear-gradient(90deg,transparent,#e5d9c4,transparent); animation:sweep 2.4s infinite; }
details { color:#687486; font-size:11px; }
summary { cursor:pointer; width:fit-content; padding:5px 0; }
summary:focus-visible { outline:1px solid #b8c6d9; outline-offset:5px; }
body::before { content:''; position:fixed; inset:0 0 auto; height:40px; -webkit-app-region:drag; }
.meta { margin-top:14px; display:grid; gap:10px; }
.meta span { display:block; margin-bottom:3px; color:#8f98a7; }
code { font-size:10px; overflow-wrap:anywhere; color:#a6b0c0; }
@keyframes appear { from { opacity:0; transform:translateY(6px); } to { opacity:1; transform:none; } }
@keyframes pulse { 50% { opacity:.35; } }
@keyframes sweep { from { transform:translateX(-110%); } to { transform:translateX(390%); } }
@media(max-width:700px) { body { align-items:flex-end; padding-bottom:12vh; } .status { width:100%; } }
[data-theme="light"] { color-scheme:light; background:#fff; color:#202124; }
[data-theme="light"] .brand { color:#202124; }
[data-theme="light"] .dot { background:#555; box-shadow:0 0 12px #0003; }
[data-theme="light"] .progress { background:#0001; }
[data-theme="light"] .progress:after { background:linear-gradient(90deg,transparent,#777,transparent); }
[data-theme="light"] :is(.message, details, .meta span, code) { color:#60646b; }
</style></head>
<body>
<section class="status">
<p class="brand">TRON</p>
<div role="status" aria-live="polite"><div class="status-row"><span class="dot"></span>${
    zh ? "正在连接 Tron" : "Connecting to Tron backend"
  }</div>
<div class="message">${escape(message).replace(/\r?\n/g, "<br>")}</div></div>
<div class="progress" aria-hidden="true"></div>
<details><summary>${
    zh ? "启动详情" : "Startup details"
  }</summary><div class="meta">
<div><span>${zh ? "数据目录" : "Data directory"}</span><code>${escape(
    dataDir
  )}</code></div>
<div><span>${zh ? "后端日志" : "Backend logs"}</span><code>${escape(
    logPath
  )}</code></div>
</div></details></section>
</body></html>`;
}
module.exports = { startupPageHtml };
