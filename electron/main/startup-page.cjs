const fs = require("node:fs");
const path = require("node:path");
const entrySceneCss = fs.readFileSync(path.join(__dirname, "entry-scene.css"), "utf8");
const { startupArtwork } = require("./startup-art.cjs");

const escape = (value) =>
  String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

// Keep these boot-time colors aligned with the built-in default skin. The
// standalone data URL cannot load the frontend's theme assets before startup.
const startupPalettes = {
  light: { background: "#f0f0f6", foreground: "#1f2329", muted: "#626773", primary: "#5861a6", tint: "#5b66cf", outline: "#e2e5eb" },
  "matrix-dark": { background: "#17181d", foreground: "#eceef2", muted: "#a0a6b3", primary: "#a1aaff", tint: "#a1aaff", outline: "#333740" }
};

function startupPalette(theme) {
  return startupPalettes[theme === "light" ? "light" : "matrix-dark"];
}

function paletteVariables(palette) {
  return Object.entries(palette).map(([key, value]) => `--${key}:${value};`).join("");
}

function startupDetails() {
  const details = document.querySelector("details");
  const trigger = details.querySelector("summary");
  document.addEventListener("pointerdown", (event) => {
    if (details.open && !details.contains(event.target)) details.open = false;
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && details.open) {
      details.open = false;
      trigger.focus();
    }
  });
}

function startupPageHtml({ message, dataDir, logPath, locale = "en", theme = "light", failed = false }) {
  const zh = locale.startsWith("zh");
  return `<!doctype html>
<html lang="${zh ? "zh-CN" : "en"}">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>TRON Harness</title>
<style>
${entrySceneCss}
:root { ${paletteVariables(startupPalette(theme))} color-scheme:${theme === "light" ? "light" : "dark"}; font-family:Inter,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; background:var(--background); color:var(--foreground); }

* { box-sizing:border-box; }
html,body { min-height:100%; margin:0; }
body::before { content:""; position:fixed; inset:0 0 auto; height:40px; z-index:2; -webkit-app-region:drag; }
.intro { position:relative; width:fit-content; max-width:100%; }
.message { color:var(--foreground); font-size:12px; line-height:1.7; max-height:120px; overflow:auto; overflow-wrap:anywhere; white-space:pre-wrap; }
.footer { margin-top:auto; padding-top:90px; animation:appear 900ms 550ms both; }
details { position:relative; width:28px; color:var(--muted); font-size:11px; }
summary { display:grid; place-items:center; width:28px; height:28px; margin-left:-5px; border-radius:50%; cursor:pointer; list-style:none; transition:color 180ms,background 180ms; }
summary::-webkit-details-marker { display:none; }
summary svg { width:16px; height:16px; opacity:.65; transition:opacity 180ms; }
summary:hover,details[open] summary { color:var(--primary); background:color-mix(in srgb,var(--primary) 7%,transparent); }
summary:hover svg,details[open] summary svg { opacity:1; }
summary:focus-visible { outline:2px solid var(--primary); outline-offset:4px; }
.meta { position:absolute; bottom:calc(100% + 12px); left:0; width:min(360px,calc(100vw - 56px)); max-height:calc(100svh - 112px); padding:20px 22px; display:grid; gap:18px; overflow:auto; border:1px solid color-mix(in srgb,var(--outline) 65%,transparent); border-radius:12px; background:color-mix(in srgb,var(--background) 88%,transparent); backdrop-filter:blur(20px); box-shadow:0 12px 40px #0000000a; }
details[open] .meta { animation:appear 180ms both; }
.meta-title { margin:0 0 2px; color:var(--foreground); font-size:12px; font-weight:500; }
.meta span { display:block; margin-bottom:6px; font-size:10px; }
code { font-family:ui-monospace,SFMono-Regular,Consolas,monospace; font-size:11px; line-height:1.6; overflow-wrap:anywhere; color:var(--foreground); user-select:text; }
@keyframes appear { from { opacity:0; transform:translateY(8px); } to { opacity:1; transform:translateY(0); } }
@media(max-height:560px) and (min-width:481px) { .footer { padding-top:48px; } }
@media(prefers-reduced-motion:reduce) { *,*::before,*::after { animation:none !important; transition:none !important; } }
</style></head>
<body class="tron-entry-scene">
<div class="tron-entry-atmosphere" aria-hidden="true"><canvas class="tron-entry-weave"></canvas></div>
<main class="startup tron-entry-layout">
<section class="intro">
<h1 class="tron-entry-brand" aria-label="TRON Harness"><span aria-hidden="true">TRON</span><span aria-hidden="true">Harness</span></h1>
<div class="tron-entry-progress" role="status" aria-label="${zh ? "正在加载" : "Loading"}"></div>
</section>
<div class="footer">
<details${failed ? " open" : ""}><summary aria-label="${zh ? "启动详情" : "Startup details"}" title="${zh ? "启动详情" : "Startup details"}"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.2v.2"/></svg></summary><div class="meta" role="region" aria-label="${zh ? "启动详情" : "Startup details"}">
<p class="meta-title">${zh ? "启动详情" : "Startup details"}</p>
<div><span>${failed ? (zh ? "启动失败" : "Startup failed") : (zh ? "启动状态" : "Startup status")}</span><div class="message" role="${failed ? "alert" : "status"}">${escape(message)}</div></div>
<div><span>${zh ? "数据目录" : "Data directory"}</span><code>${escape(dataDir)}</code></div>
<div><span>${zh ? "后端日志" : "Backend logs"}</span><code>${escape(logPath)}</code></div>
</div></details></div></main>
<script>(${startupArtwork.toString()})(document.querySelector(".tron-entry-weave"));(${startupDetails.toString()})();</script>
</body></html>`;
}
module.exports = { startupPageHtml, startupPalette };
