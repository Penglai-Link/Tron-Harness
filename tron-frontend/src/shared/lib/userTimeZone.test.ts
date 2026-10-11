import { afterEach, expect, it } from "vitest";
import {
  dateInTimeZone,
  DEFAULT_TIME_ZONE,
  isValidTimeZone,
  useTimeZoneStore,
} from "./userTimeZone";
afterEach(() => useTimeZoneStore.setState({ timeZone: DEFAULT_TIME_ZONE }));
it("uses Beijing by default and computes each user's calendar date", () => {
  expect(DEFAULT_TIME_ZONE).toBe("Asia/Shanghai");
  const now = new Date("2026-10-10T17:00:00Z");
  expect(dateInTimeZone("Asia/Shanghai", now)).toBe("2026-10-11");
  expect(dateInTimeZone("America/New_York", now)).toBe("2026-10-10");
  expect(isValidTimeZone("Not/AZone")).toBe(false);
});
it("loads server preferences into the session and falls back on invalid zones", () => {
  useTimeZoneStore.getState().setTimeZone("America/New_York");
  expect(useTimeZoneStore.getState().timeZone).toBe("America/New_York");
  useTimeZoneStore.getState().setTimeZone("Invalid/Zone");
  expect(useTimeZoneStore.getState().timeZone).toBe(DEFAULT_TIME_ZONE);
});
