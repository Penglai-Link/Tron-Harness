function appendMenuGroup(template, items) {
  const visibleItems = items.filter((item) => item.visible !== false);
  if (visibleItems.length === 0) {
    return;
  }

  if (template.length > 0) {
    template.push({ type: "separator" });
  }
  template.push(...visibleItems);
}

const ENGLISH_LABELS = Object.freeze({
  addComment: "Add Comment",
  back: "Back",
  copy: "Copy",
  copyImage: "Copy Image",
  copyImageAddress: "Copy Image Address",
  copyLinkAddress: "Copy Link Address",
  cut: "Cut",
  delete: "Delete",
  forward: "Forward",
  inspectElement: "Inspect Element",
  openLink: "Open Link in System Browser",
  paste: "Paste",
  redo: "Redo",
  reload: "Reload",
  selectAll: "Select All",
  undo: "Undo"
});

const CHINESE_LABELS = Object.freeze({
  addComment: "添加评论",
  back: "后退",
  copy: "复制",
  copyImage: "复制图像",
  copyImageAddress: "复制图像地址",
  copyLinkAddress: "复制链接地址",
  cut: "剪切",
  delete: "删除",
  forward: "前进",
  inspectElement: "检查元素",
  openLink: "在系统浏览器中打开链接",
  paste: "粘贴",
  redo: "重做",
  reload: "重新加载",
  selectAll: "全选",
  undo: "撤销"
});

function resolveWebviewContextMenuLabels(locale) {
  return String(locale || "").toLowerCase().startsWith("zh")
    ? CHINESE_LABELS
    : ENGLISH_LABELS;
}

function createWebviewCommentPayload(pageId, x, y) {
  return {
    pageId: String(pageId || "").trim(),
    x: Number.isFinite(Number(x)) ? Number(x) : 0,
    y: Number.isFinite(Number(y)) ? Number(y) : 0
  };
}

async function dispatchWebviewComment(frame, payload) {
  if (!frame || frame.isDestroyed()) {
    return false;
  }

  const pageId = JSON.stringify(String(payload?.pageId || ""));
  const x = JSON.stringify(Number(payload?.x));
  const y = JSON.stringify(Number(payload?.y));
  const script = [
    "Boolean(window.__tronDesktopInspector",
    "&& typeof window.__tronDesktopInspector.openCommentAtPoint === \"function\"",
    `&& window.__tronDesktopInspector.openCommentAtPoint(${x}, ${y}, ${pageId}) === true)`
  ].join(" ");

  try {
    return await frame.executeJavaScript(script, true) === true;
  } catch {
    return false;
  }
}

function buildWebviewContextMenuTemplate(guestContents, params, actions) {
  const template = [];
  const editFlags = params.editFlags || {};
  const linkUrl = String(params.linkURL || "");
  const sourceUrl = String(params.srcURL || "");
  const labels = resolveWebviewContextMenuLabels(actions.locale);

  appendMenuGroup(
    template,
    (params.dictionarySuggestions || []).slice(0, 5).map((suggestion) => ({
      label: suggestion,
      click: () => guestContents.replaceMisspelling(suggestion)
    }))
  );

  appendMenuGroup(template, [
    {
      label: labels.undo,
      enabled: Boolean(editFlags.canUndo),
      visible: Boolean(params.isEditable),
      click: () => guestContents.undo()
    },
    {
      label: labels.redo,
      enabled: Boolean(editFlags.canRedo),
      visible: Boolean(params.isEditable),
      click: () => guestContents.redo()
    },
    { type: "separator", visible: Boolean(params.isEditable) },
    {
      label: labels.cut,
      enabled: Boolean(editFlags.canCut),
      visible: Boolean(params.isEditable),
      click: () => guestContents.cut()
    },
    {
      label: labels.copy,
      enabled: Boolean(editFlags.canCopy || params.selectionText),
      visible: Boolean(params.isEditable || params.selectionText),
      click: () => guestContents.copy()
    },
    {
      label: labels.paste,
      enabled: Boolean(editFlags.canPaste),
      visible: Boolean(params.isEditable),
      click: () => guestContents.paste()
    },
    {
      label: labels.delete,
      enabled: Boolean(editFlags.canDelete),
      visible: Boolean(params.isEditable),
      click: () => guestContents.delete()
    },
    { type: "separator", visible: Boolean(params.isEditable) },
    {
      label: labels.selectAll,
      enabled: Boolean(editFlags.canSelectAll),
      visible: Boolean(params.isEditable || params.selectionText),
      click: () => guestContents.selectAll()
    }
  ]);

  appendMenuGroup(template, [
    {
      label: labels.openLink,
      visible: actions.isHttpUrl(linkUrl),
      click: () => actions.openExternal(linkUrl)
    },
    {
      label: labels.copyLinkAddress,
      visible: Boolean(linkUrl),
      click: () => actions.copyText(linkUrl)
    },
    {
      label: labels.copyImage,
      visible: Boolean(params.hasImageContents),
      click: () => guestContents.copyImageAt(params.x, params.y)
    },
    {
      label: labels.copyImageAddress,
      visible: Boolean(sourceUrl && params.mediaType === "image"),
      click: () => actions.copyText(sourceUrl)
    }
  ]);

  const navigationHistory = guestContents.navigationHistory;
  appendMenuGroup(template, [
    {
      label: labels.back,
      enabled: navigationHistory.canGoBack(),
      click: () => navigationHistory.goBack()
    },
    {
      label: labels.forward,
      enabled: navigationHistory.canGoForward(),
      click: () => navigationHistory.goForward()
    },
    { label: labels.reload, click: () => guestContents.reload() }
  ]);

  appendMenuGroup(template, [
    {
      label: labels.addComment,
      visible: Boolean(actions.canComment),
      click: () => actions.addComment(params.x, params.y)
    },
    {
      label: labels.inspectElement,
      visible: Boolean(actions.allowInspect),
      click: () => guestContents.inspectElement(params.x, params.y)
    }
  ]);

  return template;
}

module.exports = {
  createWebviewCommentPayload,
  dispatchWebviewComment,
  buildWebviewContextMenuTemplate,
  resolveWebviewContextMenuLabels
};
