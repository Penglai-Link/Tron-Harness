import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { fetchMemoryDomains } from "../api";
import { useAuthSession } from "@/domain/auth";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import type { MemoryTarget } from "../model";
export function useMemoryTargets() {
  const { t } = useTranslation("pages");
  const { session } = useAuthSession();
  const spaces = useWorkspaceSpaces();
  const domains = useQuery({
    queryKey: ["memory-domains", session?.user.userCode],
    queryFn: ({ signal }) =>
      fetchMemoryDomains({ includeDisabled: true }, signal),
  });
  const workspace =
    spaces.selectedSecondarySpaceId || spaces.selectedPrimarySpaceId;
  const targets = useMemo<MemoryTarget[]>(
    () => [
      {
        scope: "User",
        scopeCode: session?.user.userCode ?? "",
        label: session?.user.displayName || t("memory.simple.personal"),
      },
      ...(workspace
        ? [
            {
              scope: "Space" as const,
              scopeCode: workspace,
              label:
                spaces.selectedSecondarySpace?.name ||
                spaces.selectedPrimarySpace?.name ||
                workspace,
            },
          ]
        : []),
      ...(domains.data ?? []).map((d) => ({
        scope: "WorkDomain" as const,
        scopeCode: d.domainCode,
        label: d.name,
      })),
    ],
    [
      session,
      workspace,
      spaces.selectedSecondarySpace?.name,
      spaces.selectedPrimarySpace?.name,
      domains.data,
      t,
    ],
  );
  return { targets, domains };
}
