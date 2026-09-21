import { useEffect, useState } from "react";
import { fetchChatPanelMenuTree } from "@/domain/navigation/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  CHAT_PANEL_ROOT_MENU_CODE,
  DEFAULT_CHAT_PANEL_MENU_TREE,
} from "@/shared/constants/chatPanelMenu";
import type { NavigationRuntimeMenu } from "@/shared/types/Navigation";

function findRootMenu(menus: NavigationRuntimeMenu[]): NavigationRuntimeMenu | undefined {
  return menus.find((menu) => menu.menuCode === CHAT_PANEL_ROOT_MENU_CODE) ?? menus[0];
}

export function useChatPanelMenuTree() {
  const [menuTree, setMenuTree] = useState(DEFAULT_CHAT_PANEL_MENU_TREE);

  useEffect(() => {
    if (getServiceDataSource("admin") !== "api") return;

    const controller = new AbortController();
    void fetchChatPanelMenuTree(controller.signal)
      .then((tree) => {
        if (!controller.signal.aborted && tree?.menus?.length) {
          setMenuTree(tree);
        }
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[chat-panel] Chat panel menu tree refresh failed.", error);
      });

    return () => controller.abort();
  }, []);

  const rootMenu = findRootMenu(menuTree.menus);
  return {
    rootMenu,
    children: rootMenu?.children ?? [],
  };
}
