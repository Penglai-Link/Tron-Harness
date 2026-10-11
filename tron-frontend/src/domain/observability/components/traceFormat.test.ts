import { expect, it } from "vitest";
import { formatTraceDuration, formatTraceTokens } from "./traceFormat";

it("converts durations at second, minute and hour boundaries", () => {
  expect(
    [0, 550, 999, 1000, 1250, 60_000, 90_000, 3_600_000].map((value) =>
      formatTraceDuration(value, "en-US")
    )
  ).toEqual([
    "0 ms",
    "550 ms",
    "999 ms",
    "1 s",
    "1.25 s",
    "1 min",
    "1.5 min",
    "1 h",
  ]);
});
it("uses consistent token scales with locale-aware decimals", () => {
  expect(
    [0, 999, 1000, 12500, 1_250_000, 1_000_000_000].map((value) =>
      formatTraceTokens(value, "zh-CN")
    )
  ).toEqual(["0", "999", "1K", "12.5K", "1.25M", "1B"]);
  expect(formatTraceDuration(1250, "de-DE")).toBe("1,25 s");
});
