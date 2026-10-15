import { useSyncExternalStore } from "react";

const storageKey = "tron.app-entrance-animation";
const changedEvent = "tron.app-entrance-animation.changed";

function readPreference() {
  try { return window.localStorage.getItem(storageKey) === "true"; }
  catch { return false; }
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

export function setAppEntranceAnimation(enabled: boolean) {
  try { window.localStorage.setItem(storageKey, String(enabled)); }
  catch { return; }
  window.dispatchEvent(new Event(changedEvent));
}

/** Applies to returning users; unfinished initialization keeps its guide. */
export function useAppEntranceAnimation() {
  return useSyncExternalStore(subscribe, readPreference, () => false);
}
