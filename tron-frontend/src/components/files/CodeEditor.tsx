import { useEffect, useRef } from "react";

import { autocompletion, closeBrackets, closeBracketsKeymap, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, foldGutter, foldKeymap, indentOnInput } from "@codemirror/language";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Compartment, EditorState } from "@codemirror/state";
import {
  EditorView,
  crosshairCursor,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";

import { tronEditorTheme, tronSyntaxHighlighting } from "./editorTheme";
import { resolveLanguage } from "./language";

export interface CodeEditorProps {
  /** 作为状态缓存键的路径，让每个文件都能保留独立的撤销历史。 */
  path: string;
  /** 首次打开文件时使用的初始文档内容。 */
  value: string;
  readOnly?: boolean;
  onChange: (value: string) => void;
  onSave: () => void;
}

/**
 * 围绕单个常驻 CodeMirror 6 `EditorView` 的薄 React 封装。
 *
 * 视图只创建一次，并在切换文件时复用。每个文件的 `EditorState`（包括撤销
 * 历史和光标位置）以 `path` 为键缓存；切换文件只调用 `view.setState`，因此
 * 编辑器不会重新挂载，面板也保持响应。
 */
export function CodeEditor({
  path,
  value,
  readOnly = false,
  onChange,
  onSave,
}: CodeEditorProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  const stateCacheRef = useRef<Map<string, EditorState>>(new Map());
  const currentPathRef = useRef<string>(path);
  const loadedValueRef = useRef<Map<string, string>>(new Map());

  const languageCompartmentRef = useRef(new Compartment());
  const readOnlyCompartmentRef = useRef(new Compartment());

  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const onSaveRef = useRef(onSave);
  onSaveRef.current = onSave;
  const readOnlyRef = useRef(readOnly);
  readOnlyRef.current = readOnly;

  const createState = (filePath: string, doc: string): EditorState => {
    return EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        foldGutter(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        tronSyntaxHighlighting,
        bracketMatching(),
        closeBrackets(),
        autocompletion(),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        keymap.of([
          { key: "Mod-s", run: () => { onSaveRef.current(); return true; } },
          indentWithTab,
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...foldKeymap,
          ...completionKeymap,
        ]),
        languageCompartmentRef.current.of(resolveLanguage(filePath) ?? []),
        readOnlyCompartmentRef.current.of(EditorView.editable.of(!readOnlyRef.current)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChangeRef.current(update.state.doc.toString());
          }
        }),
        tronEditorTheme,
      ],
    });
  };

  const getOrCreateState = (filePath: string, doc: string): EditorState => {
    const cached = stateCacheRef.current.get(filePath);
    if (cached) return cached;

    const state = createState(filePath, doc);
    stateCacheRef.current.set(filePath, state);
    loadedValueRef.current.set(filePath, doc);
    return state;
  };

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const view = new EditorView({
      parent: host,
      state: getOrCreateState(path, value),
    });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // 刻意只创建一次；文件切换逻辑在下方处理。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    if (currentPathRef.current === path) {
      // 同一文件，但外部加载的内容发生了变化（初始读取已完成）。仅当传入的
      // value 与构建当前状态时使用的 value 不一致时才重建状态，以保留用户编辑。
      if (loadedValueRef.current.get(path) === value) return;

      const nextState = createState(path, value);
      stateCacheRef.current.set(path, nextState);
      loadedValueRef.current.set(path, value);
      view.setState(nextState);
      return;
    }

    // 切换文件前先保存当前文件的状态。
    stateCacheRef.current.set(currentPathRef.current, view.state);
    currentPathRef.current = path;
    view.setState(getOrCreateState(path, value));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path, value]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: readOnlyCompartmentRef.current.reconfigure(EditorView.editable.of(!readOnly)),
    });
  }, [readOnly]);

  return <div ref={hostRef} className="h-full min-h-0 overflow-hidden" />;
}
