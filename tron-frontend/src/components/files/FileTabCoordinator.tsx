import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export interface FileTabCoordinatorValue {
  /** 返回已打开指定文件的其它文件 tab 的 key，若没有则返回 null。 */
  findFileTab: (fileKey: string, excludeTabKey: string) => string | null;
  /** 记录（或清除）某个文件 tab 当前打开的文件。 */
  setActiveFile: (tabKey: string, fileKey: string | null) => void;
  /** 返回已打开指定根目录路径的其它文件 tab 的 key，若没有则返回 null。 */
  findRootTab: (rootPath: string, excludeTabKey: string) => string | null;
  /** 记录（或清除）某个文件 tab 当前打开的根目录路径。 */
  setActiveRoot: (tabKey: string, rootPath: string | null) => void;
  /** 将某个文件 tab 激活为当前 tab。 */
  focusTab: (tabKey: string) => void;
}

const FileTabCoordinatorContext = createContext<FileTabCoordinatorValue | null>(null);

/** 文件 tab 未挂载在协调器内时返回 null（例如独立使用 FilePanel 的场景）。 */
export function useFileTabCoordinator(): FileTabCoordinatorValue | null {
  return useContext(FileTabCoordinatorContext);
}

interface FileTabCoordinatorProviderProps {
  onFocusTab: (tabKey: string) => void;
  children: ReactNode;
}

export function FileTabCoordinatorProvider({
  onFocusTab,
  children,
}: FileTabCoordinatorProviderProps) {
  const [activeFiles, setActiveFiles] = useState<Record<string, string>>({});
  const [activeRoots, setActiveRoots] = useState<Record<string, string>>({});

  const findFileTab = useCallback(
    (fileKey: string, excludeTabKey: string) => {
      for (const [tabKey, activeFile] of Object.entries(activeFiles)) {
        if (tabKey !== excludeTabKey && activeFile === fileKey) return tabKey;
      }
      return null;
    },
    [activeFiles],
  );

  const setActiveFile = useCallback((tabKey: string, fileKey: string | null) => {
    setActiveFiles((previous) => {
      const next = { ...previous };
      if (fileKey === null) delete next[tabKey];
      else next[tabKey] = fileKey;
      return next;
    });
  }, []);

  const findRootTab = useCallback(
    (rootPath: string, excludeTabKey: string) => {
      for (const [tabKey, activeRoot] of Object.entries(activeRoots)) {
        if (tabKey !== excludeTabKey && activeRoot === rootPath) return tabKey;
      }
      return null;
    },
    [activeRoots],
  );

  const setActiveRoot = useCallback((tabKey: string, rootPath: string | null) => {
    setActiveRoots((previous) => {
      const next = { ...previous };
      if (rootPath === null) delete next[tabKey];
      else next[tabKey] = rootPath;
      return next;
    });
  }, []);

  const value = useMemo<FileTabCoordinatorValue>(
    () => ({ findFileTab, setActiveFile, findRootTab, setActiveRoot, focusTab: onFocusTab }),
    [findFileTab, setActiveFile, findRootTab, setActiveRoot, onFocusTab],
  );

  return (
    <FileTabCoordinatorContext.Provider value={value}>
      {children}
    </FileTabCoordinatorContext.Provider>
  );
}
