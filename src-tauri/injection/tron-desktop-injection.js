(function () {
  const BRIDGE_CHANNEL = "tron:frame-event";
  const APP_ORIGIN = __TRON_APP_ORIGIN__;
  const CONFIGURED_PAGE_ID = __TRON_PAGE_ID__;
  const ENABLED_DEFAULT = __TRON_INJECTION_ENABLED__;
  const AUTO_PAGE_ID = "__TRON_AUTO_PAGE_ID__";
  const TEST_STYLE_ID = "tron-injected-test-background";
  const ROOT_SELECTOR = "[data-tron-injected='1']";

  if (!window || !window.document) {
    return;
  }

  const clampText = (value, maxLength = 96) => {
    if (typeof value !== "string") {
      return "";
    }
    return value.trim().slice(0, maxLength);
  };

  const resolvePageId = () => {
    const configured = clampText(CONFIGURED_PAGE_ID, 96);
    if (configured && configured !== AUTO_PAGE_ID) {
      return configured;
    }

    return clampText(window.name, 96) || "unknown";
  };

  const PAGE_ID = resolvePageId();

  const frameId = (() => {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }

    return `frame-${Math.floor(Math.random() * 1e12)}`;
  })();

  const safePost = (eventType, payload) => {
    if (!injectionEnabled) {
      return;
    }

    try {
      window.parent?.postMessage(
        {
          channel: BRIDGE_CHANNEL,
          eventType,
          pageId: PAGE_ID,
          frameId,
          frameUrl: clampText(window.location.href),
          payload,
          timestamp: Date.now(),
        },
        "*",
      );
    } catch {}
  };

  const isTronAppPage = () => {
    if (!window.location || !window.location.origin) {
      return false;
    }

    return window.location.origin === APP_ORIGIN;
  };

  const installTestStyle = () => {
    if (!injectionEnabled) {
      return;
    }

    const target = document.head || document.documentElement;
    if (!target || document.getElementById(TEST_STYLE_ID)) {
      return;
    }

    const style = document.createElement("style");
    style.id = TEST_STYLE_ID;
    style.textContent = "html, body { background: #fff35a !important; }";
    target.appendChild(style);
  };

  const removeTestStyle = () => {
    document.getElementById(TEST_STYLE_ID)?.remove();
  };

  const removeShadowRoot = () => {
    document.querySelectorAll(ROOT_SELECTOR).forEach((node) => node.remove());
  };

  const installShadowRoot = () => {
    if (!injectionEnabled) {
      return;
    }

    if (document.querySelector(ROOT_SELECTOR)) {
      return;
    }

    const root = document.createElement("div");
    root.dataset.tronInjected = "1";
    const shadow = root.attachShadow({ mode: "open" });

    const style = document.createElement("style");
    style.textContent = `
      :host {
        all: initial;
      }
      .tron-badge {
        position: fixed;
        top: 12px;
        left: 12px;
        z-index: 2147483647;
        pointer-events: none;
        border-radius: 9999px;
        padding: 2px 10px;
        background: rgba(14, 165, 233, 0.18);
        color: #0ea5e9;
        font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        font-size: 12px;
        border: 1px solid rgba(14, 165, 233, 0.32);
      }
    `;

    const badge = document.createElement("div");
    badge.className = "tron-badge";
    badge.textContent = "Tron Injected";

    shadow.append(style, badge);
    document.documentElement.appendChild(root);
  };

  const applyInjectionUi = () => {
    if (!injectionEnabled) {
      removeTestStyle();
      removeShadowRoot();
      return;
    }

    installTestStyle();
    if (document.body) {
      installShadowRoot();
    } else {
      window.addEventListener("DOMContentLoaded", installShadowRoot, { once: true });
    }
  };

  const sanitizeTarget = (target) => {
    if (!target || typeof target !== "object") {
      return null;
    }

    const tagName = clampText(target.tagName, 32).toLowerCase();
    const id = clampText(target.id, 64);
    const name = clampText(target.getAttribute ? target.getAttribute("name") : "", 64);
    const className = clampText(typeof target.className === "string" ? target.className : "", 80);

    return { tagName, id, name, className };
  };

  const handleEvent = (eventType, event) => {
    safePost(eventType, {
      tag: sanitizeTarget(event?.target),
      x: Number.isFinite(event?.clientX) ? event.clientX : undefined,
      y: Number.isFinite(event?.clientY) ? event.clientY : undefined,
      value: clampText(
        event && (event.target?.value ?? event.target?.textContent ?? ""),
        160,
      ),
      key: clampText(event?.key, 40),
    });
  };

  let injectionEnabled = ENABLED_DEFAULT === true || ENABLED_DEFAULT === "true";

  if (isTronAppPage()) {
    return;
  }

  applyInjectionUi();

  const config = {
    capture: true,
    passive: true,
  };

  safePost("tron:frame-ready", {
    title: clampText(document.title),
    location: clampText(window.location.href),
    top: window.top === window.self,
  });

  document.addEventListener("click", (event) => handleEvent("tron:dom-event", event), config);
  document.addEventListener("change", (event) => handleEvent("tron:dom-event", event), config);
  document.addEventListener("input", (event) => handleEvent("tron:dom-event", event), config);

  window.addEventListener(
    "message",
    (event) => {
      const payload = event.data;
      if (!payload || typeof payload !== "object") {
        return;
      }

      if (payload.channel !== BRIDGE_CHANNEL && payload.type !== "tron:toggle-injection") {
        return;
      }

      if (typeof payload.enabled === "boolean") {
        injectionEnabled = payload.enabled;
        applyInjectionUi();
        safePost("tron:frame-ready", {
          title: clampText(document.title),
          location: clampText(window.location.href),
          top: window.top === window.self,
        });
      }
    },
    false,
  );
})();
