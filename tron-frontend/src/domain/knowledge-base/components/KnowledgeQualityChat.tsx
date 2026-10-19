import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import type { ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { AgentChatWindow } from "@/domain/chat/components/AgentChatWindow";
import { useChatHistoryLabels, useChatWorkRailLabels } from "@/domain/chat";
import { getApiAccessToken } from "@/shared/api/httpClient";
import { applyCurrentLocaleHeader } from "@/i18n";
import { Button } from "@/components/ui/button";
import { resolveQualitySession } from "../api/knowledgeQualityApi";
import { useKnowledgeUser } from "../hooks/useKnowledgeLibraries";

export function KnowledgeQualityChat({ libraryId }: { libraryId: string }) {
  const { t } = useTranslation("pages");
  const user = useKnowledgeUser();
  const session = useQuery({ queryKey: ["knowledge-quality-session", user, libraryId], queryFn: () => resolveQualitySession(libraryId), staleTime: Infinity, retry: false });
  return session.data ? <BoundChat key={session.data.sessionCode} session={session.data} libraryId={libraryId} /> : session.error ? <div role="alert">{session.error.message}<Button variant="outline" onClick={() => void session.refetch()}>{t("knowledgeQuality.retry")}</Button></div> : <p role="status">{t("knowledgeQuality.loading")}</p>;
}
function BoundChat({ session, libraryId }: { session: ChatSession; libraryId: string }) {
  const { t } = useTranslation("pages");
  const workRailLabels = useChatWorkRailLabels();
  const historyLabels = useChatHistoryLabels();
  const adapter = useMemo(() => createInternalChatAdapter({ workspaceCode: session.workspaceCode, getAccessToken: getApiAccessToken, getHeaders: () => { const headers = applyCurrentLocaleHeader(new Headers()); headers.set("X-Knowledge-Base-Code", libraryId); return headers; } }), [session.workspaceCode, libraryId]);
  const config = useMemo(() => ({ workspaceCode: session.workspaceCode ?? undefined, agentCode: session.agentCode ?? undefined, showSessionList: false, showNewSession: false, floating: false, title: session.title, workRailLabels, historyLabels, inputPlaceholder: t("knowledgeQuality.placeholder") }), [session.workspaceCode, session.agentCode, session.title, workRailLabels, historyLabels, t]);
  const createSendInput = (current: ChatSession, content: string, attachments: NonNullable<SendChatMessageInput["attachments"]>): SendChatMessageInput => ({ content, workspaceCode: current.workspaceCode, agentCode: current.agentCode, attachments, clientCapabilities: { userInputRequests: true }, modelSelection: { kind: "Auto" } });
  return <div className="flex h-full min-h-0 flex-col"><AgentChatWindow knowledgeBaseId={libraryId} showHeader={false} adapter={adapter} config={config} initialSession={session} createSendInput={createSendInput} /></div>;
}
