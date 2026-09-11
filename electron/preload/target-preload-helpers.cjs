function normalizeTargetConfiguration(payload) {
  const pageId = typeof payload?.pageId === "string"
    ? payload.pageId.trim().slice(0, 160)
    : "";
  if (!pageId) {
    return null;
  }

  return {
    commentAvailable: Boolean(payload?.commentAvailable),
    enabled: Boolean(payload?.enabled),
    pageId
  };
}

function buildPageMessageScript(payload) {
  return `window.postMessage(${JSON.stringify(payload)}, "*");`;
}

module.exports = { buildPageMessageScript, normalizeTargetConfiguration };
