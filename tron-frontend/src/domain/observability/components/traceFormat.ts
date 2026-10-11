export function formatTraceDuration(milliseconds: number, locale: string) {
  const [divisor, unit] =
    milliseconds >= 3_600_000
      ? [3_600_000, "h"]
      : milliseconds >= 60_000
      ? [60_000, "min"]
      : milliseconds >= 1_000
      ? [1_000, "s"]
      : [1, "ms"];
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 2 }).format(
    milliseconds / Number(divisor)
  )} ${unit}`;
}

export function formatTraceTokens(tokens: number, locale: string) {
  const [divisor, unit] =
    tokens >= 1_000_000_000
      ? [1_000_000_000, "B"]
      : tokens >= 1_000_000
      ? [1_000_000, "M"]
      : tokens >= 1_000
      ? [1_000, "K"]
      : [1, ""];
  return `${new Intl.NumberFormat(locale, {
    maximumFractionDigits: divisor === 1 ? 0 : 2,
  }).format(tokens / Number(divisor))}${unit}`;
}
