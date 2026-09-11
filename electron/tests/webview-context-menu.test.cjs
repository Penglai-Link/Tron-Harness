const test = require("node:test");
const assert = require("node:assert/strict");

const {
  createWebviewCommentPayload,
  dispatchWebviewComment,
  buildWebviewContextMenuTemplate,
  resolveWebviewContextMenuLabels
} = require("../main/webview-context-menu.cjs");

test("comment dispatch payload targets the configured page", () => {
  assert.deepEqual(createWebviewCommentPayload(" page-1 ", 12, 24), {
    pageId: "page-1",
    x: 12,
    y: 24
  });
});

test("comment dispatch reaches the page main world", async () => {
  const calls = [];
  const frame = {
    executeJavaScript: async (script, userGesture) => {
      const pageWindow = {
        __tronDesktopInspector: {
          openCommentAtPoint: (x, y, pageId) => {
            calls.push({ pageId, x, y });
            return true;
          }
        }
      };
      assert.equal(userGesture, true);
      return Function("window", `return (${script});`)(pageWindow);
    },
    isDestroyed: () => false
  };

  const dispatched = await dispatchWebviewComment(
    frame,
    createWebviewCommentPayload("page-1", 12, 24)
  );

  assert.equal(dispatched, true);
  assert.deepEqual(calls, [{ pageId: "page-1", x: 12, y: 24 }]);
});

test("comment dispatch reports when the page inspector is unavailable", async () => {
  const frame = {
    executeJavaScript: async (script) => Function(
      "window",
      `return (${script});`
    )({}),
    isDestroyed: () => false
  };

  const dispatched = await dispatchWebviewComment(
    frame,
    createWebviewCommentPayload("page-1", 12, 24)
  );

  assert.equal(dispatched, false);
});

function createContext(overrides = {}) {
  const calls = [];
  const navigationHistory = {
    canGoBack: () => true,
    canGoForward: () => false,
    goBack: () => calls.push(["back"]),
    goForward: () => calls.push(["forward"])
  };
  const guestContents = {
    copy: () => calls.push(["copy"]),
    copyImageAt: (x, y) => calls.push(["copy-image", x, y]),
    cut: () => calls.push(["cut"]),
    delete: () => calls.push(["delete"]),
    inspectElement: (x, y) => calls.push(["inspect", x, y]),
    navigationHistory,
    paste: () => calls.push(["paste"]),
    redo: () => calls.push(["redo"]),
    reload: () => calls.push(["reload"]),
    replaceMisspelling: (value) => calls.push(["replace", value]),
    selectAll: () => calls.push(["select-all"]),
    undo: () => calls.push(["undo"])
  };
  const params = {
    dictionarySuggestions: [],
    editFlags: {},
    hasImageContents: false,
    isEditable: false,
    linkURL: "",
    mediaType: "none",
    selectionText: "",
    srcURL: "",
    x: 12,
    y: 24,
    ...overrides
  };
  const actions = {
    addComment: (x, y) => calls.push(["add-comment", x, y]),
    allowInspect: false,
    canComment: false,
    copyText: (value) => calls.push(["copy-text", value]),
    isHttpUrl: (value) => /^https?:\/\//.test(value),
    openExternal: (value) => calls.push(["open", value])
  };

  return { actions, calls, guestContents, params };
}

function visibleItems(template) {
  return template.filter((item) => item.type !== "separator");
}

test("plain webview context includes navigation and reload", () => {
  const context = createContext();
  const items = visibleItems(
    buildWebviewContextMenuTemplate(
      context.guestContents,
      context.params,
      context.actions
    )
  );

  assert.deepEqual(items.map((item) => item.label || item.role), [
    "Back",
    "Forward",
    "Reload"
  ]);
  assert.equal(items[0].enabled, true);
  assert.equal(items[1].enabled, false);
});

test("editable context exposes editing actions using renderer flags", () => {
  const context = createContext({
    editFlags: {
      canCopy: true,
      canCut: true,
      canDelete: true,
      canPaste: false,
      canRedo: false,
      canSelectAll: true,
      canUndo: true
    },
    isEditable: true
  });
  const items = visibleItems(
    buildWebviewContextMenuTemplate(
      context.guestContents,
      context.params,
      context.actions
    )
  );
  const editItems = items.slice(0, 7);

  assert.deepEqual(editItems.map((item) => item.label), [
    "Undo",
    "Redo",
    "Cut",
    "Copy",
    "Paste",
    "Delete",
    "Select All"
  ]);
  assert.deepEqual(editItems.map((item) => item.enabled), [
    true,
    false,
    true,
    true,
    false,
    true,
    true
  ]);
});

test("link and image actions operate on the clicked target", () => {
  const context = createContext({
    hasImageContents: true,
    linkURL: "https://example.com/docs",
    mediaType: "image",
    srcURL: "https://example.com/image.png"
  });
  const items = visibleItems(
    buildWebviewContextMenuTemplate(
      context.guestContents,
      context.params,
      context.actions
    )
  );

  items.find((item) => item.label === "Open Link in System Browser").click();
  items.find((item) => item.label === "Copy Link Address").click();
  items.find((item) => item.label === "Copy Image").click();
  items.find((item) => item.label === "Copy Image Address").click();

  assert.deepEqual(context.calls, [
    ["open", "https://example.com/docs"],
    ["copy-text", "https://example.com/docs"],
    ["copy-image", 12, 24],
    ["copy-text", "https://example.com/image.png"]
  ]);
});

test("inspect element is only present when explicitly enabled", () => {
  const context = createContext();
  context.actions.allowInspect = true;
  const items = visibleItems(
    buildWebviewContextMenuTemplate(
      context.guestContents,
      context.params,
      context.actions
    )
  );

  const inspect = items.find((item) => item.label === "Inspect Element");
  assert.ok(inspect);
  inspect.click();
  assert.deepEqual(context.calls, [["inspect", 12, 24]]);
});

test("comment-capable pages can start a comment at the clicked point", () => {
  const context = createContext();
  context.actions.canComment = true;
  const items = visibleItems(
    buildWebviewContextMenuTemplate(
      context.guestContents,
      context.params,
      context.actions
    )
  );

  const addComment = items.find((item) => item.label === "Add Comment");
  assert.ok(addComment);
  addComment.click();
  assert.deepEqual(context.calls, [["add-comment", 12, 24]]);
});

test("context menu labels follow the Electron application locale", () => {
  assert.equal(resolveWebviewContextMenuLabels("zh-CN").addComment, "添加评论");
  assert.equal(resolveWebviewContextMenuLabels("zh-CN").reload, "重新加载");
  assert.equal(resolveWebviewContextMenuLabels("en-US").reload, "Reload");
});
