import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { fetchModelSelectOptions } from "@/domain/model/api";
import { useAuthStore } from "@/store/authStore";
import { contextModelDisplayName } from "../model/contextAssemblyReport";

/** Provider/model metadata follows the recorded model identity, including saved reports. */
export function useContextModelLabel(modelCode: string | null, workspaceCode = "") {
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const { t, i18n } = useTranslation("pages");
  const query = useQuery({
    queryKey: ["context-model-label", user, workspaceCode, modelCode, i18n.resolvedLanguage],
    enabled: Boolean(user && modelCode),
    queryFn: ({ signal }) => fetchModelSelectOptions(workspaceCode, signal),
    staleTime: 30_000,
    retry: false,
  });
  const label = contextModelDisplayName(modelCode, query.data ?? []);
  return label || t(query.isFetching
    ? "chat.shortTermMemory.contextPanel.loadingModel"
    : "chat.shortTermMemory.contextPanel.modelUnavailable");
}
