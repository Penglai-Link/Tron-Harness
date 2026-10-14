import { useEffect, useRef } from "react";
import { useAuthStore } from "@/store/authStore";
import { useQuery } from "@tanstack/react-query";
import { PLATFORM_INTERNAL_AGENT_SCOPE } from "@/domain/agent/model";
import { listDudes } from "@/domain/dude/api/dudeApi";
import { fetchWorkspaceInternalAgents } from "@/domain/agent/api/agentManagementApi";
import { getKnowledgeLibrary, listKnowledgeLibraries } from "../api/knowledgeBaseApi";

export function useKnowledgeUser() {
  return useAuthStore((state) => state.session?.user.userCode ?? "");
}

export function useKnowledgeViewActive() {
  const active = useRef(true);
  useEffect(() => { active.current = true; return () => { active.current = false; }; }, []);
  return active;
}

export function useKnowledgeLibraries() {
  const user = useKnowledgeUser();
  return useQuery({
    queryKey: ["knowledge-bases", user], enabled: Boolean(user),
    queryFn: ({ signal }) => listKnowledgeLibraries( signal),
  });
}

export function useKnowledgeLibrary(id: string) {
  const user = useKnowledgeUser();
  return useQuery({
    queryKey: ["knowledge-base", user, id], enabled: Boolean(user && id),
    queryFn: ({ signal }) => getKnowledgeLibrary( id, signal),
    refetchInterval: (query) => query.state.data?.documents.some((doc) => ["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(doc.status)) ? 2000 : false,
  });
}

export function useKnowledgeAgents(enabled: boolean) {
  const user = useKnowledgeUser();
  return useQuery({
    queryKey: ["knowledge-base-agents", user], enabled: Boolean(user && enabled),
    queryFn: async ({ signal }) => {
      const [agents, dudes] = await Promise.all([fetchWorkspaceInternalAgents(PLATFORM_INTERNAL_AGENT_SCOPE, signal), listDudes(signal)]);
      return [...agents.map(({ agentCode, name }) => ({ agentCode, name })), ...dudes.filter(dude => dude.enabled).map(dude => ({ agentCode: dude.dudeCode, name: dude.name }))];
    },
  });
}
