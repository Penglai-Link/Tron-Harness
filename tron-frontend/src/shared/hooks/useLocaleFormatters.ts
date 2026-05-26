import { useMemo } from "react";

import { useLocale } from "@/i18n";

interface FormatDateTimeOptions extends Intl.DateTimeFormatOptions {}
interface FormatNumberOptions extends Intl.NumberFormatOptions {}

export function useLocaleFormatters() {
  const { currentLocale } = useLocale();

  return useMemo(() => ({
    currentLocale,
    formatDateTime(value: Date | number | string, options?: FormatDateTimeOptions) {
      return new Intl.DateTimeFormat(currentLocale, options).format(new Date(value));
    },
    formatNumber(value: number, options?: FormatNumberOptions) {
      return new Intl.NumberFormat(currentLocale, options).format(value);
    },
  }), [currentLocale]);
}
