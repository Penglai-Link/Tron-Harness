import { useEffect, useRef } from "react";
import { useAuthStore } from "@/store/authStore";
import { useQuery } from "@tanstack/react-query";
import { useWorkspaceSpaces } from "@/shared";
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

export function useKnowledgeWorkspace() {
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  return selectedSecondarySpace?.workspaceCode ?? selectedPrimarySpace?.workspaceCode ?? "";
}

export function useKnowledgeLibraries(workspaceCode: string) {
  const user = useKnowledgeUser();
  return useQuery({
    queryKey: ["knowledge-bases", user, workspaceCode], enabled: Boolean(user && workspaceCode),
    queryFn: ({ signal }) => listKnowledgeLibraries(workspaceCode, signal),
  });
}

export function useKnowledgeLibrary(workspaceCode: string, id: string) {
  const user = useKnowledgeUser();
  return useQuery({
    queryKey: ["knowledge-base", user, workspaceCode, id], enabled: Boolean(user && workspaceCode && id),
    queryFn: ({ signal }) => getKnowledgeLibrary(workspaceCode, id, signal),
    refetchInterval: (query) => query.state.data?.documents.some((doc) => ["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(doc.status)) ? 2000 : false,
  });
}

export function useKnowledgeAgents(workspaceCode: string, enabled: boolean) {
  const user = useKnowledgeUser();
  return useQuery({
    queryKey: ["knowledge-base-agents", user, workspaceCode], enabled: Boolean(user && workspaceCode && enabled),
    queryFn: ({ signal }) => fetchWorkspaceInternalAgents(workspaceCode, signal),
  });
}
