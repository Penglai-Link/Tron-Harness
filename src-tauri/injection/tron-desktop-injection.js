(function () {
  const BRIDGE_CHANNEL = "tron:frame-event";
  const APP_ORIGIN = __TRON_APP_ORIGIN__;
  const CONFIGURED_PAGE_ID = __TRON_PAGE_ID__;
  const ENABLED_DEFAULT = __TRON_INJECTION_ENABLED__;
  const AUTO_PAGE_ID = "__TRON_AUTO_PAGE_ID__";
  const UI_ATTR = "data-tron-inspector-ui";
  const MAX_TEXT_LENGTH = 160;

  if (!window || !window.document || window.__tronDesktopInspectorLoaded) {
    return;
  }

  window.__tronDesktopInspectorLoaded = true;

  const clampText = (value, maxLength = 160) => {
    if (typeof value !== "string") {
      return "";
    }

    return value.replace(/\s+/g, " ").trim().slice(0, maxLength);
  };

  const clampUrl = (value) => {
    if (typeof value !== "string") {
      return "";
    }

    return value.trim().slice(0, 8192);
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

  let injectionEnabled = ENABLED_DEFAULT === true || ENABLED_DEFAULT === "true";
  let selectedElement = null;
  let hoverOverlay = null;
  let inputPanel = null;

  const safePost = (eventType, payload, options = {}) => {
    if (!injectionEnabled && !options.force) {
      return;
    }

    try {
      window.parent?.postMessage(
        {
          channel: BRIDGE_CHANNEL,
          eventType,
          pageId: PAGE_ID,
          frameId,
          frameUrl: clampUrl(window.location.href),
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

  const isInspectorUi = (element) => Boolean(
    element
      && element.closest
      && element.closest("[" + UI_ATTR + "]"),
  );

  const escapeCssIdentifier = (value) => {
    if (window.CSS && typeof window.CSS.escape === "function") {
      return window.CSS.escape(value);
    }

    return String(value).replace(/[^A-Za-z0-9_-]/g, "\\$&");
  };

  const createSelector = (element) => {
    if (!element || element.nodeType !== Node.ELEMENT_NODE) {
      return "";
    }

    if (element.id) {
      return "#" + escapeCssIdentifier(element.id);
    }

    const parts = [];
    let current = element;

    while (current && current.nodeType === Node.ELEMENT_NODE && parts.length < 5) {
      let part = current.tagName.toLowerCase();

      if (current.classList && current.classList.length) {
        part += "." + Array.from(current.classList)
          .slice(0, 2)
          .map((className) => escapeCssIdentifier(className))
          .join(".");
      }

      const siblings = current.parentElement
        ? Array.from(current.parentElement.children).filter(
          (child) => child.tagName === current.tagName,
        )
        : [];

      if (siblings.length > 1) {
        part += ":nth-of-type(" + (siblings.indexOf(current) + 1) + ")";
      }

      parts.unshift(part);
      current = current.parentElement;
    }

    return parts.join(" > ");
  };

  const summarizeText = (element) => clampText(
    element && (element.innerText || element.textContent || ""),
    MAX_TEXT_LENGTH,
  );

  const resolveNavigationUrl = (value) => {
    if (typeof value !== "string" || !value.trim()) {
      return "";
    }

    try {
      const url = new URL(value, window.location.href);
      if (url.protocol === "http:" || url.protocol === "https:") {
        return url.href;
      }
    } catch (_error) {}

    return "";
  };

  const postNavigationRequest = (url, source) => {
    safePost("tron:navigation-requested", {
      source,
      url,
    }, { force: true });
  };

  const navigateInCurrentFrame = (url, source) => {
    const targetUrl = resolveNavigationUrl(url);
    if (!targetUrl) {
      return false;
    }

    postNavigationRequest(targetUrl, source);
    window.location.href = targetUrl;
    return true;
  };

  const isBlankTarget = (target) => {
    const normalized = String(target || "").trim().toLowerCase();
    return normalized === "_blank";
  };

  const findAnchor = (target) => {
    if (!target || target.nodeType !== Node.ELEMENT_NODE) {
      return null;
    }

    return target.closest?.("a[href]") || null;
  };

  const onNavigationClick = (event) => {
    if (injectionEnabled || event.defaultPrevented || isInspectorUi(event.target)) {
      return;
    }

    const anchor = findAnchor(event.target);
    if (!anchor || !isBlankTarget(anchor.getAttribute("target"))) {
      return;
    }

    const url = resolveNavigationUrl(anchor.getAttribute("href"));
    if (!url) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
    navigateInCurrentFrame(url, "target-blank-link");
  };

  const originalWindowOpen = window.open?.bind(window);
  window.open = (url, target, features) => {
    const targetUrl = resolveNavigationUrl(url);
    if (targetUrl && (!target || isBlankTarget(target))) {
      navigateInCurrentFrame(targetUrl, "window-open");
      return window;
    }

    return originalWindowOpen ? originalWindowOpen(url, target, features) : null;
  };

  const collectAttributes = (element) => {
    const attributes = {};

    for (const attr of Array.from(element.attributes || []).slice(0, 16)) {
      attributes[attr.name] = clampText(attr.value, 240);
    }

    return attributes;
  };

  const copyComputedStyles = (source, target, options = {}) => {
    if (!source || !target || source.nodeType !== Node.ELEMENT_NODE || target.nodeType !== Node.ELEMENT_NODE) {
      return;
    }

    const style = window.getComputedStyle(source);
    let inlineStyle = "";
    for (const property of Array.from(style)) {
      inlineStyle += property + ":" + style.getPropertyValue(property) + ";";
    }
    inlineStyle += "box-sizing:" + style.boxSizing + ";";
    if (options.resetMargin) {
      inlineStyle += "margin:0;";
    }
    target.setAttribute("style", inlineStyle);

    const sourceChildren = Array.from(source.children || []);
    const targetChildren = Array.from(target.children || []);
    sourceChildren.forEach((child, index) => {
      copyComputedStyles(child, targetChildren[index], options);
    });
  };

  const serializeNodePreview = (element, width, height) => {
    const clone = element.cloneNode(true);
    copyComputedStyles(element, clone, { resetMargin: true });
    clone.removeAttribute(UI_ATTR);
    clone.querySelectorAll?.("[" + UI_ATTR + "]").forEach((node) => node.remove());

    const wrapper = document.createElement("div");
    wrapper.setAttribute("xmlns", "http://www.w3.org/1999/xhtml");
    Object.assign(wrapper.style, {
      width: width + "px",
      height: height + "px",
      overflow: "hidden",
      display: "block",
      boxSizing: "border-box",
      background: "transparent",
    });
    wrapper.appendChild(clone);

    return new XMLSerializer().serializeToString(wrapper);
  };

  const getElementPath = (root, target) => {
    const path = [];
    let current = target;

    while (current && current !== root) {
      const parent = current.parentElement;
      if (!parent) {
        return [];
      }

      path.unshift(Array.from(parent.children).indexOf(current));
      current = parent;
    }

    return current === root ? path : [];
  };

  const findElementByPath = (root, path) => {
    let current = root;

    for (const index of path) {
      current = current?.children?.[index] || null;
      if (!current) {
        return null;
      }
    }

    return current;
  };

  const annotateClonedSelection = (contextClone, selectedPath) => {
    const selectedClone = findElementByPath(contextClone, selectedPath);
    if (!selectedClone || selectedClone.nodeType !== Node.ELEMENT_NODE) {
      return;
    }

    const selectedStyle = window.getComputedStyle(selectedClone);
    if (selectedStyle.position === "static") {
      selectedClone.style.setProperty("position", "relative", "important");
    }
    if (selectedStyle.display === "inline") {
      selectedClone.style.setProperty("display", "inline-block", "important");
    }
    selectedClone.style.setProperty("outline", "3px solid #2563eb", "important");
    selectedClone.style.setProperty("outline-offset", "-3px", "important");
    selectedClone.style.setProperty("z-index", "1", "important");
  };

  const createMarkerOverlay = (markerNumber, left, top) => {
    const marker = document.createElement("span");
    marker.setAttribute("aria-hidden", "true");
    marker.textContent = String(markerNumber);
    Object.assign(marker.style, {
      position: "absolute",
      left: left + "px",
      top: top + "px",
      width: "36px",
      height: "36px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      borderRadius: "999px",
      background: "#1677ff",
      color: "#ffffff",
      font: "700 20px system-ui, sans-serif",
      lineHeight: "36px",
      textAlign: "center",
      boxSizing: "border-box",
      zIndex: "2",
      pointerEvents: "none",
    });
    marker.style.setProperty("background", "#1677ff", "important");
    marker.style.setProperty("color", "#ffffff", "important");
    marker.style.setProperty("font", "700 20px system-ui, sans-serif", "important");
    marker.style.setProperty("line-height", "36px", "important");
    return marker;
  };

  const serializeContextPreview = (
    element,
    selectedElement,
    contextWidth,
    contextHeight,
    cropLeft,
    cropTop,
    cropWidth,
    cropHeight,
    cropPaddingTop,
    cropPaddingBottom,
    selectedX,
    selectedY,
    markerNumber,
  ) => {
    const clone = element.cloneNode(true);
    copyComputedStyles(element, clone);
    clone.removeAttribute(UI_ATTR);
    clone.querySelectorAll?.("[" + UI_ATTR + "]").forEach((node) => node.remove());
    annotateClonedSelection(clone, getElementPath(element, selectedElement));

    const viewport = document.createElement("div");
    viewport.setAttribute("xmlns", "http://www.w3.org/1999/xhtml");
    Object.assign(viewport.style, {
      width: cropWidth + "px",
      height: cropHeight + cropPaddingTop + cropPaddingBottom + "px",
      overflow: "hidden",
      display: "block",
      position: "relative",
      boxSizing: "border-box",
      background: "#ffffff",
    });

    const stage = document.createElement("div");
    Object.assign(stage.style, {
      width: contextWidth + "px",
      height: contextHeight + "px",
      overflow: "visible",
      display: "block",
      boxSizing: "border-box",
      transform: "translate(" + (-cropLeft) + "px, " + (cropPaddingTop - cropTop) + "px)",
      transformOrigin: "0 0",
    });
    stage.appendChild(clone);
    viewport.appendChild(stage);
    viewport.appendChild(createMarkerOverlay(markerNumber, selectedX, cropPaddingTop + selectedY - 20));

    return new XMLSerializer().serializeToString(viewport);
  };

  const loadImage = (source) => new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("annotation image load failed"));
    image.src = source;
  });

  const drawWrappedFallbackText = (context, text, x, y, maxWidth, lineHeight) => {
    const words = String(text || "").split(/\s+/).filter(Boolean);
    let line = "";
    let cursorY = y;

    for (const word of words) {
      const candidate = line ? line + " " + word : word;
      if (context.measureText(candidate).width <= maxWidth || !line) {
        line = candidate;
        continue;
      }

      context.fillText(line, x, cursorY, maxWidth);
      line = word;
      cursorY += lineHeight;
    }

    if (line) {
      context.fillText(line, x, cursorY, maxWidth);
    }
  };

  const clampNumber = (value, min, max) => Math.max(min, Math.min(value, max));

  const findAnnotationContextElement = (element) => {
    const selectedRect = element.getBoundingClientRect();
    const selectedArea = Math.max(1, selectedRect.width * selectedRect.height);
    const selectedText = summarizeText(element);
    let current = element.parentElement;
    let fallback = element;
    let depth = 0;

    while (current && current !== document.documentElement && depth < 6) {
      const rect = current.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        fallback = current;
        const currentText = summarizeText(current);
        const onlyWrapsSelection = currentText && currentText === selectedText;
        const hasHorizontalContext = rect.width >= selectedRect.width + 120;
        const hasVerticalContext = rect.height >= selectedRect.height + 90;
        const hasAreaContext = rect.width * rect.height >= selectedArea * 2.2;
        if (!onlyWrapsSelection && (hasHorizontalContext || hasVerticalContext || hasAreaContext)) {
          return current;
        }
      }

      current = current.parentElement;
      depth += 1;
    }

    return fallback;
  };

  const createAnnotationImage = async (element, markerNumber) => {
    const rect = element.getBoundingClientRect();
    const contextElement = findAnnotationContextElement(element);
    const contextRect = contextElement.getBoundingClientRect();
    const computedStyle = window.getComputedStyle(element);
    const sourceText = summarizeText(element) || element.tagName.toLowerCase();
    const contextWidth = Math.max(1, Math.round(contextRect.width || rect.width || element.scrollWidth || 120));
    const contextHeight = Math.max(1, Math.round(contextRect.height || rect.height || element.scrollHeight || 40));
    const selectedWidth = Math.max(1, Math.round(rect.width || element.scrollWidth || 120));
    const selectedHeight = Math.max(1, Math.round(rect.height || element.scrollHeight || 40));
    const cropWidth = Math.max(260, Math.min(980, contextWidth, Math.max(520, selectedWidth + 360)));
    const cropHeight = Math.max(260, Math.min(680, contextHeight, Math.max(360, selectedHeight + 320)));
    const cropPaddingTop = 56;
    const cropPaddingBottom = 32;
    const selectedOffsetX = rect.left - contextRect.left;
    const selectedOffsetY = rect.top - contextRect.top;
    const cropLeft = clampNumber(
      selectedOffsetX - (cropWidth - selectedWidth) / 2,
      0,
      Math.max(0, contextWidth - cropWidth),
    );
    const aboveContextTarget = Math.max(150, (cropHeight - selectedHeight) * 0.62);
    const cropTop = clampNumber(
      selectedOffsetY - aboveContextTarget,
      0,
      Math.max(0, contextHeight - cropHeight),
    );
    const selectedX = selectedOffsetX - cropLeft;
    const selectedY = selectedOffsetY - cropTop;
    const imageWidth = Math.max(260, Math.round(cropWidth + 72));
    const imageHeight = Math.max(160, Math.round(cropHeight + cropPaddingTop + cropPaddingBottom + 72));
    const targetX = 36;
    const targetY = 36;
    const markerRadius = 18;
    const markerX = targetX + selectedX + markerRadius;
    const markerY = targetY + selectedY - markerRadius;
    const fontSize = Math.max(14, Math.min(24, Number.parseFloat(computedStyle.fontSize) || 16));
    const textColor = computedStyle.color || "#111827";
    const scale = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(imageWidth * scale);
    canvas.height = Math.round(imageHeight * scale);
    const context = canvas.getContext("2d");

    if (!context) {
      return {
        dataUrl: "",
        height: imageHeight,
        markerNumber,
        mimeType: "image/png",
        width: imageWidth,
      };
    }

    context.scale(scale, scale);
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, imageWidth, imageHeight);
    let previewRendered = false;
    try {
      const previewHtml = serializeContextPreview(
        contextElement,
        element,
        contextWidth,
        contextHeight,
        cropLeft,
        cropTop,
        cropWidth,
        cropHeight,
        cropPaddingTop,
        cropPaddingBottom,
        selectedX,
        selectedY,
        markerNumber,
      );
      const previewSvg = [
        `<svg xmlns="http://www.w3.org/2000/svg" width="${cropWidth}" height="${cropHeight + cropPaddingTop + cropPaddingBottom}">`,
        `<foreignObject width="100%" height="100%">`,
        previewHtml,
        `</foreignObject>`,
        `</svg>`,
      ].join("");
      const previewUrl = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(previewSvg);
      const previewImage = await loadImage(previewUrl);
      context.drawImage(previewImage, targetX, targetY, cropWidth, cropHeight + cropPaddingTop + cropPaddingBottom);
      previewRendered = true;
    } catch (_error) {
      context.fillStyle = "#ffffff";
      context.fillRect(targetX, targetY, cropWidth, cropHeight + cropPaddingTop + cropPaddingBottom);
      context.fillStyle = textColor;
      context.font = `${fontSize}px ${computedStyle.fontFamily || "system-ui, sans-serif"}`;
      context.textBaseline = "top";
      drawWrappedFallbackText(
        context,
        sourceText,
        targetX + selectedX + 8,
        targetY + cropPaddingTop + selectedY + 8,
        Math.max(40, selectedWidth - 16),
        Math.max(fontSize + 4, Number.parseFloat(computedStyle.lineHeight) || fontSize * 1.35),
      );
    }
    if (!previewRendered) {
      context.strokeStyle = "#2563eb";
      context.lineWidth = 3;
      context.strokeRect(
        targetX + selectedX + 1.5,
        targetY + cropPaddingTop + selectedY + 1.5,
        selectedWidth - 3,
        selectedHeight - 3,
      );

      context.beginPath();
      context.fillStyle = "#1677ff";
      context.arc(markerX, markerY + cropPaddingTop, markerRadius, 0, Math.PI * 2);
      context.fill();
      context.fillStyle = "#ffffff";
      context.font = "700 20px system-ui, sans-serif";
      context.textAlign = "center";
      context.textBaseline = "middle";
      context.fillText(String(markerNumber), markerX, markerY + cropPaddingTop + 1);
    }

    return {
      dataUrl: canvas.toDataURL("image/png"),
      height: imageHeight,
      markerNumber,
      mimeType: "image/png",
      width: imageWidth,
    };
  };

  const getElementPayload = (element, remark, basicInfo = {}) => {
    const rect = element.getBoundingClientRect();
    const markerNumber = 1;

    return {
      url: window.location.href,
      title: document.title,
      selector: createSelector(element),
      tagName: element.tagName.toLowerCase(),
      text: summarizeText(element),
      remark,
      rect: {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
      },
      attributes: collectAttributes(element),
      basicInfo,
      markerNumber,
    };
  };

  const ensureOverlay = () => {
    if (hoverOverlay) {
      return hoverOverlay;
    }

    hoverOverlay = document.createElement("div");
    hoverOverlay.setAttribute(UI_ATTR, "true");
    Object.assign(hoverOverlay.style, {
      position: "fixed",
      pointerEvents: "none",
      border: "2px solid #2563eb",
      background: "rgba(37, 99, 235, 0.12)",
      borderRadius: "0px",
      boxSizing: "border-box",
      display: "none",
      zIndex: "2147483646",
    });
    document.documentElement.appendChild(hoverOverlay);
    return hoverOverlay;
  };

  const updateOverlay = (element) => {
    const rect = element.getBoundingClientRect();
    const overlay = ensureOverlay();

    Object.assign(overlay.style, {
      display: "block",
      height: rect.height + "px",
      left: rect.left + "px",
      top: rect.top + "px",
      width: rect.width + "px",
    });
  };

  const removeOverlay = () => {
    if (hoverOverlay) {
      hoverOverlay.remove();
      hoverOverlay = null;
    }
  };

  const removeInputPanel = () => {
    if (inputPanel) {
      inputPanel.remove();
      inputPanel = null;
    }
  };

  const clearSelection = () => {
    selectedElement = null;
    removeInputPanel();
    removeOverlay();
  };

  const setImportantStyles = (node, styles) => {
    for (const [property, value] of Object.entries(styles)) {
      node.style.setProperty(property, value, "important");
    }
  };

  const showInputPanel = (element) => {
    removeInputPanel();

    const rect = element.getBoundingClientRect();
    const computedStyle = window.getComputedStyle(element);
    const originalInlineStyle = {
      backgroundColor: element.style.backgroundColor,
      color: element.style.color,
      fontFamily: element.style.fontFamily,
      height: element.style.height,
      marginBottom: element.style.marginBottom,
      marginLeft: element.style.marginLeft,
      marginRight: element.style.marginRight,
      marginTop: element.style.marginTop,
      opacity: element.style.opacity,
      paddingBottom: element.style.paddingBottom,
      paddingLeft: element.style.paddingLeft,
      paddingRight: element.style.paddingRight,
      paddingTop: element.style.paddingTop,
      width: element.style.width,
    };
    const panel = document.createElement("div");
    panel.setAttribute(UI_ATTR, "true");
    const panelWidth = Math.min(345, Math.max(300, window.innerWidth - 24));
    const viewportPadding = 12;
    const panelGap = 12;

    const clamp = (value, min, max) => Math.max(min, Math.min(value, max));

    const applyPanelPosition = (expanded) => {
      const currentRect = element.getBoundingClientRect();
      const panelHeight = expanded
        ? Math.min(620, Math.max(panel.offsetHeight || 0, 430))
        : Math.min(58, Math.max(panel.offsetHeight || 0, 46));
      const verticalLeft = clamp(
        currentRect.left,
        viewportPadding,
        window.innerWidth - panelWidth - viewportPadding,
      );
      const belowTop = currentRect.bottom + panelGap;
      const aboveTop = currentRect.top - panelGap - panelHeight;

      if (belowTop + panelHeight <= window.innerHeight - viewportPadding) {
        panel.style.left = verticalLeft + "px";
        panel.style.top = belowTop + "px";
        return;
      }

      if (aboveTop >= viewportPadding) {
        panel.style.left = verticalLeft + "px";
        panel.style.top = aboveTop + "px";
        return;
      }

      const rightLeft = currentRect.right + panelGap;
      const sideTop = clamp(
        currentRect.top,
        viewportPadding,
        window.innerHeight - panelHeight - viewportPadding,
      );
      if (rightLeft + panelWidth <= window.innerWidth - viewportPadding) {
        panel.style.left = rightLeft + "px";
        panel.style.top = sideTop + "px";
        return;
      }

      const leftLeft = currentRect.left - panelGap - panelWidth;
      if (leftLeft >= viewportPadding) {
        panel.style.left = leftLeft + "px";
        panel.style.top = sideTop + "px";
        return;
      }

      panel.style.left = verticalLeft + "px";
      panel.style.top = clamp(
        belowTop,
        viewportPadding,
        window.innerHeight - panelHeight - viewportPadding,
      ) + "px";
    };

    Object.assign(panel.style, {
      position: "fixed",
      left: "0",
      top: "0",
      width: panelWidth + "px",
      display: "block",
      overflow: "hidden",
      background: "#ffffff",
      border: "1px solid #e5e7eb",
      borderRadius: "999px",
      boxShadow: "0 10px 32px #d1d5db",
      boxSizing: "border-box",
      color: "#111827",
      fontFamily: "ui-sans-serif, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      zIndex: "2147483647",
    });
    setImportantStyles(panel, {
      "background": "#ffffff",
      "opacity": "1",
      "mix-blend-mode": "normal",
      "isolation": "isolate",
      "backdrop-filter": "none",
      "-webkit-backdrop-filter": "none",
    });

    const topRow = document.createElement("div");
    topRow.setAttribute(UI_ATTR, "true");
    Object.assign(topRow.style, {
      minHeight: "46px",
      display: "flex",
      alignItems: "center",
      gap: "10px",
      padding: "10px",
      boxSizing: "border-box",
    });
    setImportantStyles(topRow, {
      "background": "#ffffff",
      "opacity": "1",
      "mix-blend-mode": "normal",
    });

    const toggleButton = document.createElement("button");
    toggleButton.setAttribute(UI_ATTR, "true");
    toggleButton.setAttribute("aria-label", "显示高级设置");
    toggleButton.type = "button";
    toggleButton.innerHTML = [
      '<svg viewBox="0 0 24 24" width="26" height="26" fill="none"',
      'stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">',
      '<line x1="4" y1="7" x2="14" y2="7"></line>',
      '<line x1="20" y1="7" x2="18" y2="7"></line>',
      '<circle cx="16" cy="7" r="2"></circle>',
      '<line x1="4" y1="17" x2="10" y2="17"></line>',
      '<line x1="20" y1="17" x2="14" y2="17"></line>',
      '<circle cx="12" cy="17" r="2"></circle>',
      '</svg>',
    ].join("");
    Object.assign(toggleButton.style, {
      width: "30px",
      height: "30px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      border: "0",
      borderRadius: "9px",
      background: "transparent",
      color: "#8b8b8b",
      cursor: "pointer",
      flex: "0 0 auto",
      padding: "0",
    });
    setImportantStyles(toggleButton, {
      "opacity": "1",
    });

    const input = document.createElement("input");
    input.setAttribute(UI_ATTR, "true");
    input.placeholder = "添加评论...";
    input.type = "text";
    Object.assign(input.style, {
      minWidth: "0",
      flex: "1 1 auto",
      height: "34px",
      border: "0",
      outline: "none",
      color: "#111827",
      background: "#fff",
      fontSize: "15px",
      lineHeight: "34px",
      fontFamily: "inherit",
      boxSizing: "border-box",
      caretColor: "#2563eb",
    });
    setImportantStyles(input, {
      "background": "#ffffff",
      "opacity": "1",
      "mix-blend-mode": "normal",
    });

    const confirmButton = document.createElement("button");
    confirmButton.setAttribute(UI_ATTR, "true");
    confirmButton.setAttribute("aria-label", "确认评论");
    confirmButton.innerHTML = [
      '<svg viewBox="0 0 24 24" width="22" height="22" fill="none"',
      'stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">',
      '<polyline points="20 6 9 17 4 12"></polyline>',
      '</svg>',
    ].join("");
    Object.assign(confirmButton.style, {
      width: "34px",
      height: "34px",
      flex: "0 0 auto",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      padding: "0",
      border: "0",
      borderRadius: "50%",
      background: "#9b9b9b",
      color: "#ffffff",
      cursor: "pointer",
      fontFamily: "inherit",
      transition: "background 120ms ease, transform 120ms ease",
    });
    setImportantStyles(confirmButton, {
      "background": "#9b9b9b",
      "opacity": "1",
      "mix-blend-mode": "normal",
    });

    confirmButton.addEventListener("pointerenter", () => {
      confirmButton.style.setProperty("background", "#7f7f7f", "important");
    });
    confirmButton.addEventListener("pointerleave", () => {
      confirmButton.style.setProperty("background", "#9b9b9b", "important");
    });
    confirmButton.addEventListener("pointerdown", () => {
      confirmButton.style.transform = "scale(0.96)";
    });
    confirmButton.addEventListener("pointerup", () => {
      confirmButton.style.transform = "scale(1)";
    });

    const advancedBody = document.createElement("div");
    advancedBody.setAttribute(UI_ATTR, "true");
    Object.assign(advancedBody.style, {
      display: "none",
      background: "#ffffff",
    });
    setImportantStyles(advancedBody, {
      "background": "#ffffff",
      "opacity": "1",
      "mix-blend-mode": "normal",
    });

    const elementBar = document.createElement("div");
    elementBar.setAttribute(UI_ATTR, "true");
    Object.assign(elementBar.style, {
      height: "42px",
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      padding: "0 10px",
      borderBottom: "1px solid #e5e7eb",
      background: "#f4f4f5",
      boxSizing: "border-box",
      color: "#111827",
      fontSize: "15px",
      fontWeight: "600",
    });
    setImportantStyles(elementBar, {
      "background": "#f4f4f5",
      "opacity": "1",
      "mix-blend-mode": "normal",
    });

    const elementTag = document.createElement("div");
    elementTag.setAttribute(UI_ATTR, "true");
    elementTag.textContent = element.tagName.toLowerCase();
    elementBar.append(elementTag);

    const advancedForm = document.createElement("div");
    advancedForm.setAttribute(UI_ATTR, "true");
    Object.assign(advancedForm.style, {
      display: "grid",
      gap: "10px",
      padding: "10px",
      boxSizing: "border-box",
      height: "300px",
      overflow: "auto"
    });
    setImportantStyles(advancedForm, {
      "background": "#ffffff",
      "opacity": "1",
      "mix-blend-mode": "normal",
    });

    const makeTextInput = (value, options = {}) => {
      const field = document.createElement("input");
      field.setAttribute(UI_ATTR, "true");
      field.type = "text";
      field.value = value || "";
      field.placeholder = options.placeholder || "";
      Object.assign(field.style, {
        width: "100%",
        height: "34px",
        border: "1px solid #d1d5db",
        borderRadius: "10px",
        padding: "0 10px",
        outline: "none",
        boxShadow: "inset 0 0 0 1px #f3f4f6",
        boxSizing: "border-box",
        color: "#111827",
        background: "#ffffff",
        fontFamily: options.mono
          ? "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace"
          : "inherit",
        fontSize: "14px",
      });
      setImportantStyles(field, {
        "background": "#ffffff",
        "opacity": "1",
        "mix-blend-mode": "normal",
      });
      return field;
    };

    const makeRow = (label, control) => {
      const row = document.createElement("label");
      row.setAttribute(UI_ATTR, "true");
      Object.assign(row.style, {
        display: "grid",
        gridTemplateColumns: "82px 1fr",
        alignItems: "center",
        gap: "10px",
        minHeight: "36px",
        color: "#676b70",
        fontSize: "14px",
      });

      const text = document.createElement("span");
      text.setAttribute(UI_ATTR, "true");
      text.textContent = label;
      row.append(text, control);
      return row;
    };

    const makeDivider = () => {
      const divider = document.createElement("div");
      divider.setAttribute(UI_ATTR, "true");
      Object.assign(divider.style, {
        height: "1px",
        background: "#e5e7eb",
      });
      return divider;
    };

    const normalizeColorToHex = (value, fallback = "#000000") => {
      const rawValue = String(value || "").trim();
      if (!rawValue) {
        return fallback;
      }

      const shortHex = rawValue.match(/^#([0-9a-f]{3})$/i);
      if (shortHex) {
        return "#" + shortHex[1].split("").map((part) => part + part).join("").toLowerCase();
      }

      if (/^#[0-9a-f]{6}$/i.test(rawValue)) {
        return rawValue.toLowerCase();
      }

      const rgbMatch = rawValue.match(/^rgba?\(([^)]+)\)$/i);
      if (rgbMatch) {
        const channels = rgbMatch[1]
          .split(/[\s,\/]+/)
          .filter(Boolean)
          .slice(0, 3)
          .map((channel) => Math.max(0, Math.min(255, Number.parseFloat(channel))));

        if (channels.length === 3 && channels.every((channel) => Number.isFinite(channel))) {
          return "#" + channels
            .map((channel) => Math.round(channel).toString(16).padStart(2, "0"))
            .join("");
        }
      }

      try {
        const canvas = document.createElement("canvas");
        const context = canvas.getContext("2d");
        if (context) {
          context.fillStyle = fallback;
          context.fillStyle = rawValue;
          return normalizeColorToHex(context.fillStyle, fallback);
        }
      } catch (_error) {}

      return fallback;
    };

    const makeColorControl = (value) => {
      const wrapper = document.createElement("div");
      wrapper.setAttribute(UI_ATTR, "true");
      Object.assign(wrapper.style, {
        height: "34px",
        display: "flex",
        alignItems: "center",
        gap: "10px",
        border: "1px solid #d1d5db",
        borderRadius: "10px",
        padding: "0 10px",
        boxShadow: "inset 0 0 0 1px #f3f4f6",
        boxSizing: "border-box",
        background: "#ffffff",
      });
      setImportantStyles(wrapper, {
        "background": "#ffffff",
        "opacity": "1",
        "mix-blend-mode": "normal",
      });

      const picker = document.createElement("input");
      picker.setAttribute(UI_ATTR, "true");
      picker.type = "color";
      picker.value = normalizeColorToHex(value);
      Object.assign(picker.style, {
        width: "24px",
        height: "24px",
        flex: "0 0 auto",
        border: "0",
        borderRadius: "10px",
        padding: "0",
        background: "#fff",
        boxSizing: "border-box",
        cursor: "pointer",
      });
      setImportantStyles(picker, {
        "background": "#ffffff",
        "opacity": "1",
      });

      const field = makeTextInput(value, { mono: true, compact: true });
      Object.assign(field.style, {
        height: "32px",
        border: "0",
        boxShadow: "none",
        padding: "0",
        color: "#8b8b8b",
      });
      setImportantStyles(field, {
        "background": "#ffffff",
        "opacity": "1",
        "mix-blend-mode": "normal",
      });

      picker.addEventListener("input", () => {
        field.value = picker.value;
      });
      field.addEventListener("input", () => {
        picker.value = normalizeColorToHex(field.value, picker.value);
      });

      wrapper.append(picker, field);
      return { wrapper, field, picker };
    };

    const normalizeNumberInput = (value) => {
      const parsed = Number.parseFloat(String(value || "").replace("px", ""));
      return Number.isFinite(parsed) ? String(Math.round(parsed)) : "";
    };

    const readPixelValue = (value, fallback = "") => normalizeNumberInput(value) || normalizeNumberInput(fallback);

    const makePixelInput = (value) => {
      const wrapper = document.createElement("div");
      wrapper.setAttribute(UI_ATTR, "true");
      Object.assign(wrapper.style, {
        height: "34px",
        display: "grid",
        gridTemplateColumns: "1fr auto",
        alignItems: "center",
        gap: "6px",
        border: "1px solid #d1d5db",
        borderRadius: "12px",
        padding: "0 10px",
        boxShadow: "inset 0 0 0 1px #f3f4f6",
        boxSizing: "border-box",
        background: "#ffffff",
      });
      setImportantStyles(wrapper, {
        "background": "#ffffff",
        "opacity": "1",
        "mix-blend-mode": "normal",
      });

      const field = document.createElement("input");
      field.setAttribute(UI_ATTR, "true");
      field.type = "number";
      field.min = "0";
      field.step = "1";
      field.value = readPixelValue(value);
      Object.assign(field.style, {
        width: "100%",
        minWidth: "0",
        height: "32px",
        border: "0",
        padding: "0",
        outline: "none",
        background: "#ffffff",
        color: "#8b8b8b",
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        fontSize: "14px",
        boxSizing: "border-box",
      });
      setImportantStyles(field, {
        "background": "#ffffff",
        "opacity": "1",
        "mix-blend-mode": "normal",
      });

      const unit = document.createElement("span");
      unit.setAttribute(UI_ATTR, "true");
      unit.textContent = "px";
      Object.assign(unit.style, {
        color: "#8b8b8b",
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        fontSize: "14px",
      });

      wrapper.append(field, unit);
      return { wrapper, field };
    };

    const makeQuadPixelControl = (values) => {
      const wrapper = document.createElement("div");
      wrapper.setAttribute(UI_ATTR, "true");
      Object.assign(wrapper.style, {
        height: "34px",
        display: "grid",
        gridTemplateColumns: "repeat(4, 1fr)",
        overflow: "hidden",
        border: "1px solid #d1d5db",
        borderRadius: "12px",
        boxShadow: "inset 0 0 0 1px #f3f4f6",
        boxSizing: "border-box",
        background: "#ffffff",
      });
      setImportantStyles(wrapper, {
        "background": "#ffffff",
        "opacity": "1",
        "mix-blend-mode": "normal",
      });

      const fields = values.map((value, index) => {
        const field = document.createElement("input");
        field.setAttribute(UI_ATTR, "true");
        field.type = "number";
        field.min = "0";
        field.step = "1";
        field.value = readPixelValue(value);
        field.title = ["Top", "Right", "Bottom", "Left"][index];
        Object.assign(field.style, {
          width: "100%",
          minWidth: "0",
          height: "32px",
          border: "0",
          borderLeft: index === 0 ? "0" : "1px solid #edf0f2",
          padding: "0 4px",
          outline: "none",
          textAlign: "center",
          background: "#ffffff",
          color: "#8b8b8b",
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
          fontSize: "14px",
          boxSizing: "border-box",
        });
        setImportantStyles(field, {
          "background": "#ffffff",
          "opacity": "1",
          "mix-blend-mode": "normal",
        });
        wrapper.append(field);
        return field;
      });

      return { wrapper, fields };
    };

    const textField = makeTextInput(summarizeText(element), { placeholder: "文本内容" });
    const textColor = makeColorControl(computedStyle.color);
    const backgroundColor = makeColorControl(computedStyle.backgroundColor);
    const opacityField = makeTextInput(computedStyle.opacity || "1", { mono: true });
    const fontField = makeTextInput(computedStyle.fontFamily || "", { mono: true });
    const widthControl = makePixelInput(computedStyle.width);
    const heightControl = makePixelInput(computedStyle.height);
    const paddingControl = makeQuadPixelControl([
      computedStyle.paddingTop,
      computedStyle.paddingRight,
      computedStyle.paddingBottom,
      computedStyle.paddingLeft,
    ]);
    const marginControl = makeQuadPixelControl([
      computedStyle.marginTop,
      computedStyle.marginRight,
      computedStyle.marginBottom,
      computedStyle.marginLeft,
    ]);

    const applyText = () => {
      if ("value" in element) {
        element.value = textField.value;
        element.dispatchEvent(new Event("input", { bubbles: true }));
        element.dispatchEvent(new Event("change", { bubbles: true }));
      } else {
        element.textContent = textField.value;
      }
      updateOverlay(element);
    };

    const applyStyleValue = (property, value, fallbackValue = "") => {
      const trimmedValue = value.trim();
      element.style[property] = trimmedValue || fallbackValue;
      updateOverlay(element);
    };

    textField.addEventListener("input", applyText);
    const applyTextColor = () => {
      applyStyleValue("color", textColor.field.value, originalInlineStyle.color);
    };
    const applyBackgroundColor = () => {
      applyStyleValue("backgroundColor", backgroundColor.field.value, originalInlineStyle.backgroundColor);
    };

    textColor.field.addEventListener("input", applyTextColor);
    textColor.picker.addEventListener("input", applyTextColor);
    backgroundColor.field.addEventListener("input", applyBackgroundColor);
    backgroundColor.picker.addEventListener("input", applyBackgroundColor);
    opacityField.addEventListener("input", () => {
      const value = opacityField.value.trim();
      const parsed = Number(value);
      if (!value) {
        element.style.opacity = originalInlineStyle.opacity;
      } else if (Number.isFinite(parsed)) {
        element.style.opacity = String(Math.max(0, Math.min(1, parsed)));
      }
      updateOverlay(element);
    });
    fontField.addEventListener("input", () => {
      applyStyleValue("fontFamily", fontField.value, originalInlineStyle.fontFamily);
    });

    const applyPixelStyle = (property, field, fallbackValue = "") => {
      const value = field.value.trim();
      if (!value) {
        element.style[property] = fallbackValue;
      } else {
        const parsed = Number.parseFloat(value);
        if (Number.isFinite(parsed)) {
          element.style[property] = Math.max(0, parsed) + "px";
        }
      }
      updateOverlay(element);
      if (advancedVisible) {
        requestAnimationFrame(() => applyPanelPosition(true));
      }
    };

    const applyQuadStyle = (properties, control, fallbacks) => {
      properties.forEach((property, index) => {
        applyPixelStyle(property, control.fields[index], fallbacks[index] || "");
      });
    };

    widthControl.field.addEventListener("input", () => {
      applyPixelStyle("width", widthControl.field, originalInlineStyle.width);
    });
    heightControl.field.addEventListener("input", () => {
      applyPixelStyle("height", heightControl.field, originalInlineStyle.height);
    });
    paddingControl.fields.forEach((field) => {
      field.addEventListener("input", () => {
        applyQuadStyle(
          ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"],
          paddingControl,
          [
            originalInlineStyle.paddingTop,
            originalInlineStyle.paddingRight,
            originalInlineStyle.paddingBottom,
            originalInlineStyle.paddingLeft,
          ],
        );
      });
    });
    marginControl.fields.forEach((field) => {
      field.addEventListener("input", () => {
        applyQuadStyle(
          ["marginTop", "marginRight", "marginBottom", "marginLeft"],
          marginControl,
          [
            originalInlineStyle.marginTop,
            originalInlineStyle.marginRight,
            originalInlineStyle.marginBottom,
            originalInlineStyle.marginLeft,
          ],
        );
      });
    });

    advancedForm.append(
      makeRow("文本", textField),
      makeDivider(),
      makeRow("文本颜色", textColor.wrapper),
      makeRow("背景", backgroundColor.wrapper),
      makeRow("Opacity", opacityField),
      makeDivider(),
      makeRow("字体", fontField),
      makeDivider(),
      makeRow("宽度", widthControl.wrapper),
      makeRow("高度", heightControl.wrapper),
      makeRow("内边距", paddingControl.wrapper),
      makeRow("外边距", marginControl.wrapper),
    );
    advancedBody.append(elementBar, advancedForm);

    const readBasicInfo = () => ({
      text: textField.value.trim(),
      textColor: textColor.field.value.trim(),
      background: backgroundColor.field.value.trim(),
      opacity: opacityField.value.trim(),
      fontFamily: fontField.value.trim(),
      width: widthControl.field.value.trim(),
      height: heightControl.field.value.trim(),
      padding: paddingControl.fields.map((field) => field.value.trim()).join(" "),
      margin: marginControl.fields.map((field) => field.value.trim()).join(" "),
    });

    const setAdvancedVisible = (visible) => {
      advancedBody.style.display = visible ? "block" : "none";
      panel.style.borderRadius = visible ? "20px" : "999px";
      topRow.style.borderBottom = visible ? "1px solid #e5e7eb" : "0";
      toggleButton.style.background = visible ? "#f3f4f6" : "#fff";
      toggleButton.style.color = visible ? "#111827" : "#8b8b8b";
      toggleButton.setAttribute("aria-label", visible ? "隐藏高级设置" : "显示高级设置");
      requestAnimationFrame(() => {
        applyPanelPosition(visible);
      });
    };

    let advancedVisible = false;
    toggleButton.addEventListener("click", () => {
      advancedVisible = !advancedVisible;
      setAdvancedVisible(advancedVisible);
    });

    const submit = async () => {
      const payload = getElementPayload(element, input.value.trim(), readBasicInfo());
      payload.annotationImage = await createAnnotationImage(element, payload.markerNumber);
      safePost("tron:node-inspected", payload);
      clearSelection();
    };

    confirmButton.addEventListener("click", () => {
      void submit();
    });
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        void submit();
      }
    });

    topRow.append(toggleButton, input, confirmButton);
    panel.append(topRow, advancedBody);
    document.documentElement.appendChild(panel);
    inputPanel = panel;
    setImportantStyles(panel, {
      "background": "#ffffff",
      "opacity": "1",
    });
    setImportantStyles(input, {
      "background": "#ffffff",
      "opacity": "1",
    });
    applyPanelPosition(false);
    setTimeout(() => input.focus(), 0);
  };

  const onPointerMove = (event) => {
    if (!injectionEnabled || selectedElement) {
      return;
    }

    const element = document.elementFromPoint(event.clientX, event.clientY);

    if (!element || isInspectorUi(element)) {
      return;
    }

    updateOverlay(element);
  };

  const onPointerDown = (event) => {
    if (!injectionEnabled) {
      return;
    }

    if (isInspectorUi(event.target)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();

    const element = document.elementFromPoint(event.clientX, event.clientY);
    if (!element) {
      return;
    }

    selectedElement = element;
    updateOverlay(element);
    showInputPanel(element);
  };

  const onClick = (event) => {
    if (!injectionEnabled || isInspectorUi(event.target)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
  };

  const onKeyDown = (event) => {
    if (!injectionEnabled || event.key !== "Escape") {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();

    if (selectedElement) {
      clearSelection();
      return;
    }

    safePost("tron:inspection-exited", { reason: "escape" }, { force: true });
    injectionEnabled = false;
    applyInspectorState();
  };

  const enableInspector = () => {
    document.addEventListener("pointermove", onPointerMove, true);
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("click", onClick, true);
    window.addEventListener("keydown", onKeyDown, true);
  };

  const disableInspector = () => {
    clearSelection();
    document.removeEventListener("pointermove", onPointerMove, true);
    document.removeEventListener("pointerdown", onPointerDown, true);
    document.removeEventListener("click", onClick, true);
    window.removeEventListener("keydown", onKeyDown, true);
  };

  const applyInspectorState = () => {
    if (injectionEnabled) {
      enableInspector();
      safePost("tron:frame-ready", {
        title: clampText(document.title),
        location: clampText(window.location.href),
        top: window.top === window.self,
      });
      return;
    }

    disableInspector();
  };

  if (isTronAppPage()) {
    return;
  }

  document.addEventListener("click", onNavigationClick, true);
  applyInspectorState();

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
        const wasEnabled = injectionEnabled;
        injectionEnabled = payload.enabled;

        if (wasEnabled !== injectionEnabled) {
          applyInspectorState();
        } else if (injectionEnabled) {
          safePost("tron:frame-ready", {
            title: clampText(document.title),
            location: clampText(window.location.href),
            top: window.top === window.self,
          });
        }
      }
    },
    false,
  );
})();
