// Observe presentation readiness without changing authentication or navigation.
module.exports = function observeStartupAnimation({ ipcRenderer, window, document }) {
  window.addEventListener("DOMContentLoaded", () => {
    let theme;
    let ready = false;
    const update = () => {
      const nextTheme = document.documentElement.dataset.theme;
      if (["light", "matrix-dark"].includes(nextTheme) && theme !== nextTheme) {
        theme = nextTheme;
        ipcRenderer.send("tron-animation:theme", theme);
      }
      // A full-window startup view can make Chromium mark this page hidden.
      // The animation stops while hidden, so waiting for its first frame would
      // keep the view in place forever. Reveal the mounted scene in that case.
      const hiddenScene = document.hidden && document.querySelector(".black-hole-scene");
      if (!ready && (hiddenScene || document.querySelector(".black-hole-ready, .black-hole-scene[data-renderer='fallback'], [data-tron-entry-ready]"))) {
        ready = true;
        if (hiddenScene) ipcRenderer.send("tron-animation:ready");
        else window.requestAnimationFrame(() => ipcRenderer.send("tron-animation:ready"));
      }
    };
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { subtree: true, childList: true, attributes: true, attributeFilter: ["class", "data-theme", "data-renderer", "data-tron-entry-ready"] });
    update();
    window.addEventListener("pagehide", () => observer.disconnect(), { once: true });
  }, { once: true });
};
