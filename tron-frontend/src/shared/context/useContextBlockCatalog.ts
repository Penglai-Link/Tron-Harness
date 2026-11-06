import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { requestApi } from "@/shared/api/httpClient";
import { useAuthStore } from "@/store/authStore";
import type { ContextBlockUpdateMode, ContextCompressionMetadata } from "./contextBlockPresentation";

export interface ContextBlockMetadata {
  id: string;
  displayName: string;
  description: string;
  updateMode: ContextBlockUpdateMode;
  compression?: ContextCompressionMetadata | null;
  providerId: string | null;
}

export type ContextBlockCatalog = ReadonlyMap<string, ContextBlockMetadata>;
const emptyCatalog: ContextBlockCatalog = new Map();

/** The backend resolves contributor-owned translations for the visible language. */
export function useContextBlockCatalog(): ContextBlockCatalog {
  const { i18n } = useTranslation();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const user = useAuthStore(state => state.session?.user.userCode ?? "");
  const query = useQuery({
    queryKey: ["context-block-metadata", user, locale],
    queryFn: ({ signal }) => requestApi<ContextBlockMetadata[]>("GET /api/v1/context-blocks", { signal, headers: { "Accept-Language": locale } }),
    select: (blocks: ContextBlockMetadata[]) => new Map(blocks.map(block => [block.id, block])),
    staleTime: 60_000,
    retry: 1,
  });
  return query.data ?? emptyCatalog;
}
