import { ShinyText } from "@/shared/motion";
import {
  createContext,
  lazy,
  Suspense,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import { FileTabCoordinatorProvider } from "@/components/files/FileTabCoordinator";
import { useFileTabsStore } from "@/store/fileTabsStore";
import { panelTabId, useTabStore } from "@/store/tabStore";
import { cn } from "@/lib/utils";

const FilePanel = lazy(() =>
  import("@/components/files/FilePanel").then((module) => ({ default: module.FilePanel })),
);

function canReuseFileTab(tabKey: string, sourceTabKey: string) {
  const owners = useFileTabsStore.getState().fileTabOwners;
  return owners[tabKey] === owners[sourceTabKey];
}

const FilePanelMountContext = createContext<
  (element: HTMLElement | null) => void
>(() => undefined);

/** 让 RightPanelContainer 把全局 FilePanel 挂载到自己的根节点。 */
export function useFilePanelTarget() {
  return useContext(FilePanelMountContext);
}

function FilePanelHost() {
  const { t } = useTranslation("common");
  const fileTabKeys = useFileTabsStore((state) => state.fileTabKeys);
  const owner = useTabStore((state) => panelTabId(state.activeTabId, state.workspaceCode));
  const activeFileTabKey = useFileTabsStore((state) => state.activeKeysByOwner[owner] ?? null);

  if (!fileTabKeys.length) return null;

  const isActive = activeFileTabKey !== null;

  return (
    <div
      aria-hidden={!isActive}
      className={cn(
        "pointer-events-none absolute inset-0 flex flex-col",
        isActive ? "z-20" : "z-0 invisible",
      )}
    >
      {/* 与路由级 tab 栏等高，避免遮挡 tab 栏；点击穿透到下层 tab 栏。 */}
      <div className="h-11 shrink-0 pointer-events-none" />
      <div className="pointer-events-auto relative min-h-0 flex-1">
        {fileTabKeys.map((key) => {
          const isCurrent = key === activeFileTabKey;
          return (
            <div
              key={key}
              className={cn(
                isCurrent
                  ? "relative h-full w-full"
                  : "absolute inset-0 invisible pointer-events-none",
              )}
            >
              <Suspense
                fallback={
                  <div className="flex h-full items-center justify-center text-sm text-[var(--color-on-surface-variant)]">
                    <ShinyText>{t("files.loading")}</ShinyText>
                  </div>
                }
              >
                <FilePanel tabKey={key} />
              </Suspense>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * 在应用根部挂载一次 FilePanel，并把它移动到当前右侧面板的根节点。
 *
 * FilePanel 的内容（选中的目录、打开的文件、编辑器状态）保存在该组件实例中。
 * 关键点：portal 的容器是一个稳定的 div，切换路由（跨 Shell）时只是把这个 div
 * append 到新的右侧面板根节点，容器本身不变，因此 FilePanel 不会被 React 重挂载，
 * 文件状态在根路径（无路由）与有路由页面之间保持一致。
 */
export function FilePanelHostProvider({ children }: { children: ReactNode }) {
  const [container] = useState(() => {
    const element = document.createElement("div");
    element.className = "pointer-events-none absolute inset-0";
    return element;
  });
  const setActiveFileTab = useFileTabsStore((state) => state.setActiveFileTab);

  const mountFilePanel = useCallback(
    (element: HTMLElement | null) => {
      // Skip hidden hosts (e.g. the mobile aside is `lg:hidden` on desktop).
      // `offsetParent` is null when the element or any ancestor has
      // `display: none`, which prevents the mobile RightPanelContainer from
      // stealing the portal container away from the visible desktop host.
      if (
        element &&
        container.parentElement !== element &&
        element.offsetParent !== null
      ) {
        element.appendChild(container);
      }
    },
    [container],
  );

  return (
    <FilePanelMountContext.Provider value={mountFilePanel}>
      {children}
      <FileTabCoordinatorProvider onFocusTab={setActiveFileTab} canReuseTab={canReuseFileTab}>
        {createPortal(<FilePanelHost />, container)}
      </FileTabCoordinatorProvider>
    </FilePanelMountContext.Provider>
  );
}
