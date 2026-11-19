import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ToolExecutionMode } from "@tron/chat-core";
import { useToast } from "@/components/ui/toast";
import type { ChatSession } from "../model";
import { updateChatSession } from "./useChatSessionGroupList";

const defaultModeStorageKey = "tron.chat.executionMode.default";

/** A session a command is about to send to; the sender may hold a runtime copy. */
interface PlanModeTarget {
  executionMode?: ToolExecutionMode;
  sessionCode: string;
}

function readDefaultMode(): ToolExecutionMode {
  try {
    const stored = localStorage.getItem(defaultModeStorageKey);
    if (stored === "AUTO_APPROVE" || stored === "PLAN") return stored;
    return "CONTROLLED";
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
  const rememberDefault = useCallback((next: ToolExecutionMode) => {
    setDefaultMode(next);
    try { localStorage.setItem(defaultModeStorageKey, next); } catch { /* Storage may be unavailable. */ }
  }, []);
  /**
   * Persists a mode on an explicit session instead of the currently selected
   * one. The `/plan` command uses it before sending, because the run is admitted
   * with the mode already stored on the session.
   */
  const saveMode = useCallback(async (target: PlanModeTarget | undefined, next: ToolExecutionMode) => {
    if (savePending.current) return false;
    if (!target || (target.executionMode ?? "CONTROLLED") === next) {
      rememberDefault(next);
      return true;
    }
    savePending.current = true;
    try {
      const updated = await updateChatSession(target.sessionCode, { executionMode: next });
      rememberDefault(next);
      onSessionUpdated(updated);
      return true;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chat.management.composer.executionModeSaveFailed"));
      return false;
    } finally {
      savePending.current = false;
    }
  }, [onSessionUpdated, rememberDefault, t, toast]);
  const changeMode = useCallback(async (next: ToolExecutionMode) => {
    if (savePending.current || next === mode) return;
    if (draftCode || !session) {
      rememberDefault(next);
      return;
    }
    setSaving(true);
    try {
      await saveMode(session, next);
    } finally {
      setSaving(false);
    }
  }, [draftCode, mode, rememberDefault, saveMode, session]);
  const applyMode = useCallback((target: PlanModeTarget, next: ToolExecutionMode) => saveMode(target, next), [saveMode]);
  return { mode, saving, changeMode, applyMode };
}
