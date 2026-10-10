import { useSyncExternalStore } from "react";

export type MotionPreference = "system" | "reduced" | "off";
const storageKey = "tron.motion.mode";
const changedEvent = "tron.motion.changed";

function readPreference(): MotionPreference {
  try {
    const value = window.localStorage.getItem(storageKey);
    return value === "reduced" || value === "off" ? value : "system";
  } catch {
    return "system";
  }
}

function subscribe(callback: () => void) {
  const onStorage = (event: StorageEvent) => {
    if (event.key === storageKey || event.key === null) callback();
  };
  window.addEventListener("storage", onStorage);
  window.addEventListener(changedEvent, callback);
  return () => {
    window.removeEventListener("storage", onStorage);
    window.removeEventListener(changedEvent, callback);
  };
}

export function setMotionPreference(value: MotionPreference) {
  try {
    window.localStorage.setItem(storageKey, value);
  } catch {
    return;
  }
  window.dispatchEvent(new Event(changedEvent));
}

export function useMotionPreference() {
  return useSyncExternalStore(subscribe, readPreference, () => "system" as const);
}
