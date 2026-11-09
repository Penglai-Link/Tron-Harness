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
      if (!ready && document.querySelector("[data-tron-entry-ready]")) {
        ready = true;
        window.requestAnimationFrame(() => ipcRenderer.send("tron-animation:ready"));
      }
    };
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { subtree: true, childList: true, attributes: true, attributeFilter: ["class", "data-theme", "data-renderer", "data-tron-entry-ready"] });
    update();
    window.addEventListener("pagehide", () => observer.disconnect(), { once: true });
  }, { once: true });
};
