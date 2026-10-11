import { create } from "zustand";
export const DEFAULT_TIME_ZONE = "Asia/Shanghai";
export function isValidTimeZone(value: string) {
  try {
    new Intl.DateTimeFormat("en", { timeZone: value });
    return Boolean(value);
  } catch {
    return false;
  }
}
export function dateInTimeZone(timeZone: string, now = new Date()): string {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(now);
  const part = (type: string) => parts.find((p) => p.type === type)!.value;
  return `${part("year")}-${part("month")}-${part("day")}`;
}
export const useTimeZoneStore = create<{
  timeZone: string;
  setTimeZone: (zone: string) => void;
}>((set) => ({
  timeZone: DEFAULT_TIME_ZONE,
  setTimeZone: (zone) =>
    set({ timeZone: isValidTimeZone(zone) ? zone : DEFAULT_TIME_ZONE }),
}));
export function getUserTimeZone() {
  return useTimeZoneStore.getState().timeZone;
}
