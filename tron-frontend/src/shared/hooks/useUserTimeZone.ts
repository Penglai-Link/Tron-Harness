import { useTimeZoneStore } from "../lib/userTimeZone";
export function useUserTimeZone() {
  return { timeZone: useTimeZoneStore((s) => s.timeZone) };
}
