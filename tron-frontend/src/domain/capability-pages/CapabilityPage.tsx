import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { i18n } from "@/i18n";
import { requestApi } from "@/shared/api/httpClient";
import { Button } from "@/components/ui/button";
import { buildPageDocument, type PageBundle } from "./document";

export function CapabilityPage() {
  const { packId = "", pageId = "" } = useParams();
  const { t } = useTranslation("pages");
  const frame = useRef<HTMLIFrameElement>(null);
  const [revision, setRevision] = useState(0);
  const [loaded, setLoaded] = useState<{ html: string; title: Record<string, string> }>();
  const [error, setError] = useState("");
  const [ready, setReady] = useState(false);
  const zh = i18n.language.startsWith("zh");
  useEffect(() => {
    const controller = new AbortController();
    let alive = true;
    let dispose: (() => void) | undefined;
    let port: MessagePort | undefined;
    let identity: string | undefined;
    let checking = false;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const instance = crypto.randomUUID();
    const endpoint = "GET /api/v1/capabilities/packs/{id}/pages/{page}";
    const options = { pathParams: { id: packId, page: pageId }, signal: controller.signal };
    setLoaded(undefined); setError(""); setReady(false);
    const stop = (message: string) => { if (!alive) return; setLoaded(undefined); setError(message); port?.close(); dispose?.(); dispose = undefined; };
    const context = () => {
      const style = getComputedStyle(document.body);
      const tokens: Record<string, string> = {};
      for (let index = 0; index < style.length; index++) { const name = style.item(index); if (name.startsWith("--color-")) tokens[name] = style.getPropertyValue(name); }
      return { packId, pageId, locale: i18n.language, appearance: document.documentElement.classList.contains("dark") || document.documentElement.dataset.theme?.includes("dark") ? "dark" : "light", tokens };
    };
    const send = () => port?.postMessage({ type: "context", context: context() });
    const connect = (event: MessageEvent) => {
      if (!alive || port || event.source !== frame.current?.contentWindow || event.data?.type !== "tron-page-ready" || event.data?.instance !== instance) return;
      if (event.data.protocolVersion !== 1) { stop("protocol"); return; }
      const channel = new MessageChannel(); port = channel.port1;
      port.onmessage = event => { if (alive && event.data?.type === "error") { clearTimeout(timeout); stop("load"); return; } if (alive && event.data?.type === "ready" && event.data.protocolVersion === 1) { clearTimeout(timeout); setReady(true); send(); } };
      port.start(); frame.current?.contentWindow?.postMessage({ type: "tron-page-connect", instance }, "*", [channel.port2]);
    };
    window.addEventListener("message", connect);
    requestApi<PageBundle>(endpoint, options).then(bundle => {
      if (!alive) return;
      const document = buildPageDocument(bundle, instance); dispose = document.dispose;
      identity = `${bundle.version}:${bundle.digest}`;
      setLoaded({ html: document.html, title: bundle.title });
      timeout = setTimeout(() => stop("load"), 15000);
    }).catch(() => { if (alive && !controller.signal.aborted) stop("load"); });
    const verify = async () => {
      if (!alive || !identity || checking) return;
      checking = true;
      try { const bundle = await requestApi<PageBundle>(endpoint, options); if (alive && `${bundle.version}:${bundle.digest}` !== identity) stop("changed"); }
      catch { if (alive && !controller.signal.aborted) stop("changed"); }
      finally { checking = false; }
    };
    const observer = new MutationObserver(send);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class", "style", "data-theme", "data-skin", "lang"] });
    observer.observe(document.body, { attributes: true, attributeFilter: ["class", "style", "data-skin"] });
    i18n.on("languageChanged", send);
    window.addEventListener("tron:navigation-updated", verify);
    window.addEventListener("focus", verify);
    return () => { alive = false; controller.abort(); clearTimeout(timeout); port?.close(); dispose?.(); observer.disconnect(); i18n.off("languageChanged", send); window.removeEventListener("message", connect); window.removeEventListener("tron:navigation-updated", verify); window.removeEventListener("focus", verify); };
  }, [packId, pageId, revision]);
  return <div className="flex h-full min-h-80 flex-col gap-3">
    {error ? <div role="alert" className="p-6"><p>{t(`capabilityPage.${error}`)}</p><Button className="mt-3" onClick={() => setRevision(value => value + 1)}>{t("capabilityPage.retry")}</Button></div> : <>
      {!ready && <p role="status">{t("capabilityPage.loading")}</p>}
      {loaded && <iframe key="capability-frame" ref={frame} title={loaded.title[zh ? "zh-CN" : "en-US"] ?? pageId} sandbox="allow-scripts" referrerPolicy="no-referrer" srcDoc={loaded.html} className="min-h-80 w-full flex-1 border-0" />}
    </>}
  </div>;
}
