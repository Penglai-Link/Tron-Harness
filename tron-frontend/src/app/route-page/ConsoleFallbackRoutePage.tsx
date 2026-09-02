import { AuthUserMenuPanel, useAuthSession } from "@/domain/auth";
import { tronNavigation, TronNavItem, TronShell } from "@/shared";
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
  const { session } = useAuthSession();

  return (
    <TronShell
      activeItem={resolveActiveItem(pathname)?.label ?? "Apps"}
      roleCodes={session?.user.roleCodes}
      userMenuContent={<AuthUserMenuPanel />}
    >
      <ConsoleFallbackPage />
    </TronShell>
  );
}
