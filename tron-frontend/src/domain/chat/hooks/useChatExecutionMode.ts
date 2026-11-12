import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ToolExecutionMode } from "@tron/chat-core";
import { useToast } from "@/components/ui/toast";
import type { ChatSession } from "../model";
import { updateChatSession } from "./useChatSessionGroupList";

const defaultModeStorageKey = "tron.chat.executionMode.default";

function readDefaultMode(): ToolExecutionMode {
  try {
    return localStorage.getItem(defaultModeStorageKey) === "AUTO_APPROVE" ? "AUTO_APPROVE" : "CONTROLLED";
  } catch {
    return "CONTROLLED";
  }
}

export function useChatExecutionMode(
  session: ChatSession | undefined,
  draftCode: string,
  onSessionUpdated: (session: ChatSession) => void,
) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [defaultMode, setDefaultMode] = useState<ToolExecutionMode>(readDefaultMode);
  const [saving, setSaving] = useState(false);
  const savePending = useRef(false);
  const mode = draftCode ? defaultMode : session?.executionMode ?? defaultMode;
  const changeMode = useCallback(async (next: ToolExecutionMode) => {
    if (savePending.current || next === mode) return;
    const saveDefault = () => {
      setDefaultMode(next);
      try { localStorage.setItem(defaultModeStorageKey, next); } catch { /* Storage may be unavailable. */ }
    };
    if (draftCode) {
      saveDefault();
      return;
    }
    if (!session) return;
    savePending.current = true;
    setSaving(true);
    try {
      const updated = await updateChatSession(session.sessionCode, { executionMode: next });
      saveDefault();
      onSessionUpdated(updated);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.management.composer.executionModeSaveFailed"));
    } finally {
      savePending.current = false;
      setSaving(false);
    }
  }, [draftCode, mode, onSessionUpdated, session, t, toast]);
  return { mode, saving, changeMode };
}
