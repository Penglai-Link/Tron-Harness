import { create } from "zustand";

/** 文件面板 tab key 的固定前缀，用于在右侧面板中识别文件 tab。 */
export const FILE_TAB_KEY_PREFIX = "page-files:";

interface FileTabsState {
  /** 已打开的文件 tab key 列表，跨路由共享。 */
  fileTabKeys: string[];
  /** 每个文件 tab 的标题（选择文件夹后为文件夹名），未设置时回退默认文案。 */
  fileTabLabels: Record<string, string>;
  /** 当前激活的文件 tab key；null 表示当前激活的是路由级 tab（属性/网页）。 */
  activeFileTabKey: string | null;
  openFileTab: () => string;
  closeFileTab: (key: string) => void;
  setActiveFileTab: (key: string | null) => void;
  setFileTabLabel: (key: string, label: string) => void;
}

let nextFileTabId = 1;

export const useFileTabsStore = create<FileTabsState>((set) => ({
  fileTabKeys: [],
  fileTabLabels: {},
  activeFileTabKey: null,

  openFileTab: () => {
    const key = `${FILE_TAB_KEY_PREFIX}${nextFileTabId++}`;
    set((state) => ({
      fileTabKeys: [...state.fileTabKeys, key],
      activeFileTabKey: key,
    }));
    return key;
  },

  closeFileTab: (key) => {
    set((state) => {
      const fileTabKeys = state.fileTabKeys.filter((item) => item !== key);
      const wasActive = state.activeFileTabKey === key;
      const activeFileTabKey = wasActive
        ? (fileTabKeys[fileTabKeys.length - 1] ?? null)
        : state.activeFileTabKey;
      const fileTabLabels = { ...state.fileTabLabels };
      delete fileTabLabels[key];
      return { fileTabKeys, fileTabLabels, activeFileTabKey };
    });
  },

  setActiveFileTab: (key) => {
    set({ activeFileTabKey: key });
  },

  setFileTabLabel: (key, label) => {
    set((state) => {
      if (state.fileTabLabels[key] === label) return state;
      return { fileTabLabels: { ...state.fileTabLabels, [key]: label } };
    });
  },
}));
