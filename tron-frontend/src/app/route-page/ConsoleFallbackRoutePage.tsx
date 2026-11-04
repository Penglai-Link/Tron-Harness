import { AuthUserMenuPanel } from "@/domain/auth";
import { tronNavigation, type TronNavItem } from "@/shared";
import { TronShell } from "@/shared/layout/Shell";
import { useLocation } from "react-router";
import ConsoleFallbackPage from "./ConsoleFallbackPage";

export const consoleNavigationItems = tronNavigation.flatMap((section) =>
  flattenNavigationItems(section.items),
);

function flattenNavigationItems(items: TronNavItem[]): TronNavItem[] {
  return items.flatMap((item) => [
    item,
    ...flattenNavigationItems(item.children ?? []),
  ]);
}

export function resolveActiveItem(pathname: string) {
  const matches = consoleNavigationItems.filter(
    (item) => item.href === pathname,
  );
  return matches.find((item) => !item.children?.length) ?? matches[0];
}

export default function ConsoleFallbackRoutePage() {
  const { pathname } = useLocation();
  return (
    <TronShell
      activeItem={resolveActiveItem(pathname)?.label ?? "Apps"}
      userMenuContent={<AuthUserMenuPanel />}
    >
      <ConsoleFallbackPage />
    </TronShell>
  );
}
