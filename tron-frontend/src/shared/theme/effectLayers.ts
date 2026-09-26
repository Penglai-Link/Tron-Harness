/** Public theme contract. Keep the Rust and theme_pack validators in sync. */
export const effectLayerNames = ["global", "app-shell", "sidebar", "page", "panel", "card", "chat", "chat-message", "dialog"] as const;
export type EffectLayerName = typeof effectLayerNames[number];

export function effectSelectorName(selector: string): EffectLayerName | "*" | null {
  const match = /^\[data-theme-effect-layer(?:\s*=\s*(["']?)([a-z-]+)\1)?\s*\](?:::before|::after)?$/.exec(selector.trim());
  if (!match) return null;
  if (!match[2]) return "*";
  return effectLayerNames.includes(match[2] as EffectLayerName) ? match[2] as EffectLayerName : null;
}

let stopCurrent: (() => void) | undefined;

/** Own only generated nodes; business components expose anchors, never theme geometry. */
export function mountEffectLayers(names: ReadonlySet<EffectLayerName>): () => void {
  stopCurrent?.();
  if (!names.size) return () => {};
  const entries = new Map<HTMLElement, { viewport: HTMLElement; name: string; positioned: boolean }>();
  let stopped = false;
  let frame = 0;
  const update = (host: HTMLElement) => {
    const entry = entries.get(host);
    if (!entry || host === document.body) return;
    // Counter the host's own scrolling. Ancestor scrolling/motion follows naturally.
    const values = { top: `${host.scrollTop}px`, left: `${host.scrollLeft}px`, width: `${host.clientWidth}px`, height: `${host.clientHeight}px` };
    for (const [key, value] of Object.entries(values)) entry.viewport.style.setProperty(key, value, "important");
  };
  const resize = new ResizeObserver((records) => records.forEach(({ target }) => update(target as HTMLElement)));
  const remove = (host: HTMLElement) => {
    const entry = entries.get(host);
    if (!entry) return;
    resize.unobserve(host);
    entry.viewport.remove();
    if (entry.positioned) delete host.dataset.themeEffectPositioned;
    entries.delete(host);
  };
  const add = (host: HTMLElement, name: string) => {
    const viewport = document.createElement("span");
    viewport.dataset.themeEffectViewport = name;
    viewport.setAttribute("aria-hidden", "true");
    viewport.inert = true;
    // This clipping/positioning boundary belongs to Tron, not theme CSS. It prevents
    // oversized or animated artwork from creating scrollbars or escaping a dialog.
    const style = { all: "initial", display: "block", position: host === document.body ? "fixed" : "absolute", inset: "0", overflow: "clip", contain: "strict", "pointer-events": "none", "border-radius": "inherit", "z-index": host === document.body ? "50" : "10" };
    for (const [key, value] of Object.entries(style)) viewport.style.setProperty(key, value, "important");
    const layer = document.createElement("span");
    layer.dataset.themeEffectLayer = name;
    layer.setAttribute("aria-hidden", "true");
    viewport.appendChild(layer);
    const positioned = host !== document.body && getComputedStyle(host).position === "static";
    if (positioned) host.dataset.themeEffectPositioned = "";
    entries.set(host, { viewport, name, positioned });
    host.appendChild(viewport);
    update(host);
    if (host !== document.body) resize.observe(host);
  };
  const reconcile = () => {
    frame = 0;
    if (stopped) return;
    // Do not observe our own insertions/removals and schedule an endless RAF loop.
    mutations.disconnect();
    for (const [host, entry] of entries) {
      if (!host.isConnected || (host !== document.body && host.dataset.themeAnchor !== entry.name)) remove(host);
      else if (entry.viewport.parentElement !== host) { remove(host); }
    }
    document.querySelectorAll<HTMLElement>("[data-theme-anchor]").forEach((host) => {
      const name = host.dataset.themeAnchor as EffectLayerName;
      if (names.has(name) && !entries.has(host)) add(host, name);
    });
    mutations.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-theme-anchor"] });
  };
  const mutations = new MutationObserver((records) => {
    const hasAnchor = (node: Node) => node instanceof Element && (
      node.hasAttribute("data-theme-anchor") || node.querySelector("[data-theme-anchor]") !== null
    );
    // Streaming text and unrelated controls do not require a document-wide scan.
    const changed = records.some((record) => record.type === "attributes"
      || [...record.addedNodes, ...record.removedNodes].some(hasAnchor)
      || (record.target instanceof HTMLElement && entries.has(record.target)
        && entries.get(record.target)?.viewport.parentElement !== record.target));
    if (changed && !frame) frame = requestAnimationFrame(reconcile);
  });
  const onScroll = (event: Event) => { if (event.target instanceof HTMLElement) update(event.target); };
  const stop = () => {
    if (stopped) return;
    stopped = true;
    cancelAnimationFrame(frame);
    mutations.disconnect();
    resize.disconnect();
    document.removeEventListener("scroll", onScroll, true);
    for (const host of entries.keys()) remove(host);
    if (stopCurrent === stop) stopCurrent = undefined;
  };
  stopCurrent = stop;
  if (names.has("global")) add(document.body, "global");
  if ([...names].some((name) => name !== "global")) {
    reconcile();
    document.addEventListener("scroll", onScroll, true);
  }
  return stop;
}
