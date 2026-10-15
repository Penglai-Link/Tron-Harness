import {
  useCallback,
  useEffect,
  useState,
} from "react";

const storageKey = "tron.aguiWorkReplayPreferences";
const changedEventName = "tron.aguiWorkReplayPreferences.changed";

export const defaultAguiWorkReplayTurnLimit = 5;
export const minimumAguiWorkReplayTurnLimit = 1;
export const maximumAguiWorkReplayTurnLimit = 20;

export interface AguiWorkReplayPreferences {
  recentTurnLimit: number;
  showReasoningCards: boolean;
  runningEnterSubmitMode: "steer" | "queue";
}

const defaultPreferences: AguiWorkReplayPreferences = {
  recentTurnLimit: defaultAguiWorkReplayTurnLimit,
  showReasoningCards: false,
  runningEnterSubmitMode: "steer",
};

function normalizeTurnLimit(value: unknown) {
  const numericValue = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numericValue)) return defaultAguiWorkReplayTurnLimit;
  return Math.min(
    maximumAguiWorkReplayTurnLimit,
    Math.max(minimumAguiWorkReplayTurnLimit, Math.round(numericValue)),
  );
}

function normalizePreferences(value: unknown): AguiWorkReplayPreferences {
  if (typeof value !== "object" || value === null) {
    return { ...defaultPreferences };
  }
  const candidate = value as Partial<AguiWorkReplayPreferences>;
  return {
    recentTurnLimit: normalizeTurnLimit(candidate.recentTurnLimit),
    showReasoningCards: candidate.showReasoningCards === true,
    runningEnterSubmitMode: candidate.runningEnterSubmitMode === "queue" ? "queue" : "steer",
  };
}

function readPreferences() {
  if (typeof window === "undefined") {
    return { ...defaultPreferences };
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    return raw ? normalizePreferences(JSON.parse(raw)) : { ...defaultPreferences };
  } catch {
    return { ...defaultPreferences };
  }
}

function writePreferences(preferences: AguiWorkReplayPreferences) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(storageKey, JSON.stringify(normalizePreferences(preferences)));
  window.dispatchEvent(new CustomEvent(changedEventName));
}

export function useAguiWorkReplayPreferences() {
  const [preferences, setPreferenceState] = useState<AguiWorkReplayPreferences>(() => readPreferences());

  const setRecentTurnLimit = useCallback((recentTurnLimit: number) => {
    setPreferenceState((current) => {
      const next = normalizePreferences({ ...current, recentTurnLimit });
      writePreferences(next);
      return next;
    });
  }, []);

  const setShowReasoningCards = useCallback((showReasoningCards: boolean) => {
    setPreferenceState((current) => {
      const next = normalizePreferences({ ...current, showReasoningCards });
      writePreferences(next);
      return next;
    });
  }, []);

  const setRunningEnterSubmitMode = useCallback((runningEnterSubmitMode: "steer" | "queue") => {
    setPreferenceState((current) => {
      const next = normalizePreferences({ ...current, runningEnterSubmitMode });
      writePreferences(next);
      return next;
    });
  }, []);

  useEffect(() => {
    function refreshPreferences() {
      setPreferenceState(readPreferences());
    }

    function handleStorage(event: StorageEvent) {
      if (event.key === storageKey) refreshPreferences();
    }

    window.addEventListener("storage", handleStorage);
    window.addEventListener(changedEventName, refreshPreferences);

    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener(changedEventName, refreshPreferences);
    };
  }, []);

  return {
    preferences,
    setRecentTurnLimit,
    setShowReasoningCards,
    setRunningEnterSubmitMode,
  };
}
