export interface PageBundle {
  packId: string; pageId: string; version: string; digest: string;
  title: Record<string, string>; entry: string; assets: Record<string, string>;
}
const types: Record<string, string> = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif", webp: "image/webp", svg: "image/svg+xml", ico: "image/x-icon", woff: "font/woff", woff2: "font/woff2", ttf: "font/ttf", otf: "font/otf" };
export function buildPageDocument(bundle: PageBundle, instance: string) {
  const resources: Record<string, { type: string; data: string }> = {};
  const text = (name: string) => new TextDecoder().decode(Uint8Array.from(atob(bundle.assets[name] ?? ""), c => c.charCodeAt(0)));
  const resolve = (ref: string, from: string) => {
    if (!ref || /^(?:[a-z]+:|\/\/|\/|#)/i.test(ref) || /[?#%\\]/.test(ref)) throw new Error(`Unsupported resource: ${ref}`);
    const path = new URL(ref, `https://pack.invalid/${from}`).pathname.slice(1);
    if (!Object.hasOwn(bundle.assets, path)) throw new Error(`Missing resource: ${path}`);
    return path;
  };
  const blobs = new Map<string, string>();
  const asset = (ref: string, from: string) => {
    const name = resolve(ref, from);
    if (blobs.has(name)) return blobs.get(name)!;
    const type = types[name.split(".").pop() ?? ""];
    if (!type) throw new Error(`Unsupported resource type: ${name}`);
    const url = `tron-asset-${crypto.randomUUID()}`; resources[url] = { type, data: bundle.assets[name] }; blobs.set(name, url); return url;
  };
  const css = (value: string, from: string) => {
    if (/@import|<\/style/i.test(value)) throw new Error("CSS imports are unsupported");
    return value.replace(/url\(\s*(['"]?)(.*?)\1\s*\)/gi, (_, _quote, ref: string) => `url("${asset(ref, from)}")`);
  };
  try {
    const doc = new DOMParser().parseFromString(text(bundle.entry), "text/html");
    if (doc.querySelector("base, iframe, frame, object, embed, form, meta[http-equiv], audio, video, source")) throw new Error("Unsupported page element");
    let scripts = 0;
    for (const element of Array.from(doc.querySelectorAll("*"))) {
      for (const attr of Array.from(element.attributes)) {
        if (/^on/i.test(attr.name) || ["srcdoc", "srcset", "action", "formaction", "ping", "background"].includes(attr.name)) throw new Error(`Unsupported attribute: ${attr.name}`);
      }
      if (element.hasAttribute("style")) element.setAttribute("style", css(element.getAttribute("style")!, bundle.entry));
      if (element.tagName === "SCRIPT") {
        if (++scripts > 1) throw new Error("Only one script entry is supported");
        const source = element.getAttribute("src");
        if (source && !source.endsWith(".js")) throw new Error("Script entry must be JavaScript");
        const code = source ? text(resolve(source, bundle.entry)) : element.textContent ?? "";
        if (/\bimport\s*(?:\(|[{'"*])|\bexport\s|<\/script/i.test(code)) throw new Error("Use a single IIFE bundle without imports");
        const script = doc.createElement("script"); script.textContent = code; element.replaceWith(script);
      } else if (element.tagName === "LINK") {
        if (element.getAttribute("rel") !== "stylesheet") throw new Error("Only stylesheet links are supported");
        const name = resolve(element.getAttribute("href") ?? "", bundle.entry);
        const style = doc.createElement("style"); style.textContent = css(text(name), name); element.replaceWith(style);
      } else {
        if (element.tagName === "STYLE") element.textContent = css(element.textContent ?? "", bundle.entry);
        if (element.hasAttribute("src")) element.setAttribute("src", asset(element.getAttribute("src")!, bundle.entry));
        if (element.hasAttribute("href")) {
          const href = element.getAttribute("href")!;
          if (!href.startsWith("#")) throw new Error("Only in-page links are supported");
        }
      }
    }
    const nonce = crypto.randomUUID();
    for (const script of Array.from(doc.scripts)) script.setAttribute("nonce", nonce);
    const policy = doc.createElement("meta"); policy.httpEquiv = "Content-Security-Policy";
    policy.content = `default-src 'none'; script-src 'nonce-${nonce}'; style-src 'unsafe-inline'; img-src blob:; font-src blob:; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'`;
    doc.head.prepend(policy);
    const bridge = doc.createElement("script"); bridge.setAttribute("nonce", nonce);
    bridge.textContent = `(() => { const instance = ${JSON.stringify(instance)}; let connected = false; let channel; let failed = false; const fail = () => { failed = true; channel?.postMessage({type:"error"}); }; window.addEventListener("error", fail); window.addEventListener("unhandledrejection", fail); const listeners = new Set(); let context; window.tronPage = Object.freeze({ protocolVersion: 1, get context() { return context; }, onContext(fn) { listeners.add(fn); if(context) fn(context); return () => listeners.delete(fn); } }); window.addEventListener('message', e => { if(connected || e.source !== parent || e.data?.type !== 'tron-page-connect' || e.data?.instance !== instance || !e.ports[0]) return; connected = true; const port = e.ports[0]; channel = port; port.onmessage = event => { if(event.data?.type !== 'context') return; context = event.data.context; document.documentElement.lang = context.locale; document.documentElement.style.colorScheme = context.appearance; for(const [key,value] of Object.entries(context.tokens)) document.documentElement.style.setProperty(key, value); listeners.forEach(fn => fn(context)); }; port.start(); port.postMessage(failed ? {type:'error'} : {type:'ready', protocolVersion:1}); }); parent.postMessage({type:'tron-page-ready', protocolVersion:1, instance}, '*'); })();`;
    policy.after(bridge);
    const html = `<!doctype html>${doc.documentElement.outerHTML}`;
    // Blob URLs belong to the opaque sandbox origin, so create them there.
    const encoded = btoa(Array.from(new TextEncoder().encode(html), byte => String.fromCharCode(byte)).join(""));
    const boot = `(() => { const resources = ${JSON.stringify(resources)}; const urls = []; let html = new TextDecoder().decode(Uint8Array.from(atob(${JSON.stringify(encoded)}), c => c.charCodeAt(0))); for (const [key, asset] of Object.entries(resources)) { const url = URL.createObjectURL(new Blob([Uint8Array.from(atob(asset.data), c => c.charCodeAt(0))], {type:asset.type})); urls.push(url); html = html.split(key).join(url); } document.open(); document.write(html); document.close(); window.addEventListener('pagehide', () => urls.forEach(url => URL.revokeObjectURL(url)), {once:true}); })();`;
    return { html: `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="${policy.content}"><script nonce="${nonce}">${boot}</script></head><body></body></html>`, dispose: () => {} };
  } catch (error) { throw error; }
}
