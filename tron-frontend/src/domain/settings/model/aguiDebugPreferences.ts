import {
  useCallback,
  useEffect,
  useState,
} from "react";

export type AguiDebugMode = "core" | "all" | "custom";

export interface AguiDebugPreferences {
  customEventTypes: string[];
  mode: AguiDebugMode;
}

interface AguiDebugModeOption {
  description: string;
  label: string;
  value: AguiDebugMode;
}

interface AguiEventTypeOption {
  description: string;
  label: string;
  value: string;
}

const storageKey = "tron.aguiDebugPreferences";
const changedEventName = "tron.aguiDebugPreferences.changed";

export const aguiDebugModeOptions: readonly AguiDebugModeOption[] = [
  {
    description: "Show the live run, reasoning, tool, and activity cards operators normally need.",
    label: "Core cards",
    value: "core",
  },
  {
    description: "Show every live AG-UI event card, including low-level text message stream events.",
    label: "All cards",
    value: "all",
  },
  {
    description: "Choose the exact live AG-UI event types that should appear in the chat transcript.",
    label: "Custom",
    value: "custom",
  },
];

export const aguiEventTypeOptions: readonly AguiEventTypeOption[] = [
  { value: "RUN_STARTED", label: "Run started", description: "Live AG-UI run start events." },
  { value: "RUN_FINISHED", label: "Run finished", description: "Live AG-UI run completion events." },
  { value: "RUN_ERROR", label: "Run error", description: "Live AG-UI run error events." },
  { value: "REASONING_START", label: "Reasoning start", description: "Reasoning block start markers." },
  { value: "REASONING_MESSAGE_START", label: "Reasoning message start", description: "Reasoning message start markers." },
  { value: "REASONING_MESSAGE_CONTENT", label: "Reasoning message content", description: "Reasoning stream text deltas." },
  { value: "REASONING_MESSAGE_END", label: "Reasoning message end", description: "Reasoning message end markers." },
  { value: "REASONING_END", label: "Reasoning end", description: "Reasoning block end markers." },
  { value: "TOOL_CALL_START", label: "Tool call start", description: "Tool invocation start markers." },
  { value: "TOOL_CALL_ARGS", label: "Tool call arguments", description: "Tool invocation argument deltas." },
  { value: "TOOL_CALL_RESULT", label: "Tool call result", description: "Tool result payload events." },
  { value: "TOOL_CALL_END", label: "Tool call end", description: "Tool invocation end markers." },
  { value: "ACTIVITY_DELTA", label: "Activity delta", description: "Runtime progress and activity updates." },
  { value: "TEXT_MESSAGE_START", label: "Text message start", description: "Assistant text stream start markers." },
  { value: "TEXT_MESSAGE_CONTENT", label: "Text message content", description: "Assistant text stream content deltas." },
  { value: "TEXT_MESSAGE_END", label: "Text message end", description: "Assistant text stream end markers." },
];

const coreEventTypes = [
  "RUN_STARTED",
  "RUN_FINISHED",
  "RUN_ERROR",
  "REASONING_START",
  "REASONING_END",
  "TOOL_CALL_START",
  "TOOL_CALL_RESULT",
  "TOOL_CALL_END",
  "ACTIVITY_DELTA",
] as const;

const defaultPreferences: AguiDebugPreferences = {
  customEventTypes: [...coreEventTypes],
  mode: "core",
};

function normalizeEventType(value: string) {
  return value.trim().toUpperCase();
}

function isAguiDebugMode(value: unknown): value is AguiDebugMode {
  return value === "core" || value === "all" || value === "custom";
}

function uniqueKnownEventTypes(values: unknown): string[] {
  if (!Array.isArray(values)) return [...defaultPreferences.customEventTypes];

  const knownByNormalizedType = new Map(
    aguiEventTypeOptions.map((option) => [normalizeEventType(option.value), option.value]),
  );
  const selected = new Set<string>();

  values.forEach((value) => {
    if (typeof value !== "string") return;
    const knownValue = knownByNormalizedType.get(normalizeEventType(value));
    if (knownValue) selected.add(knownValue);
  });

  return Array.from(selected);
}

function normalizePreferences(value: unknown): AguiDebugPreferences {
  if (typeof value !== "object" || value === null) return { ...defaultPreferences };
  const candidate = value as Partial<AguiDebugPreferences>;

  return {
    customEventTypes: uniqueKnownEventTypes(candidate.customEventTypes),
    mode: isAguiDebugMode(candidate.mode) ? candidate.mode : defaultPreferences.mode,
  };
}

function readPreferences() {
  if (typeof window === "undefined") return { ...defaultPreferences };

  try {
    const raw = window.localStorage.getItem(storageKey);
    return raw ? normalizePreferences(JSON.parse(raw)) : { ...defaultPreferences };
  } catch {
    return { ...defaultPreferences };
  }
}

function writePreferences(preferences: AguiDebugPreferences) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(storageKey, JSON.stringify(preferences));
  window.dispatchEvent(new CustomEvent(changedEventName));
}

export function shouldShowAguiEventType(type: string, preferences: AguiDebugPreferences) {
  const normalizedType = normalizeEventType(type);
  if (!normalizedType) return false;
  const selected = resolveAguiDebugEventTypes(preferences);
  return selected.some((eventType) => normalizeEventType(eventType) === normalizedType);
}

export function resolveAguiDebugEventTypes(preferences: AguiDebugPreferences) {
  if (preferences.mode === "all") return aguiEventTypeOptions.map((option) => option.value);
  if (preferences.mode === "core") return [...coreEventTypes];
  return preferences.customEventTypes;
}

export function useAguiDebugPreferences() {
  const [preferences, setPreferenceState] = useState<AguiDebugPreferences>(() => readPreferences());

  const setPreferences = useCallback((updater: AguiDebugPreferences | ((current: AguiDebugPreferences) => AguiDebugPreferences)) => {
    setPreferenceState((current) => {
      const next = normalizePreferences(typeof updater === "function" ? updater(current) : updater);
      writePreferences(next);
      return next;
    });
  }, []);

  const setMode = useCallback((mode: AguiDebugMode) => {
    setPreferences((current) => ({
      ...current,
      mode,
    }));
  }, [setPreferences]);

  const toggleCustomEventType = useCallback((eventType: string, checked: boolean) => {
    setPreferences((current) => {
      const selected = new Set(current.customEventTypes);
      if (checked) selected.add(eventType);
      else selected.delete(eventType);

      return {
        ...current,
        customEventTypes: Array.from(selected),
      };
    });
  }, [setPreferences]);

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
    setMode,
    setPreferences,
    toggleCustomEventType,
  };
}
