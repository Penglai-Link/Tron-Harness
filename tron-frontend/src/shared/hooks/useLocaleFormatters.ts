import { useMemo } from "react";

import { useLocale } from "@/i18n";
import { useUserTimeZone } from "./useUserTimeZone";

interface FormatDateTimeOptions extends Intl.DateTimeFormatOptions {}
interface FormatNumberOptions extends Intl.NumberFormatOptions {}

export function useLocaleFormatters() {
  const { currentLocale } = useLocale();
  const { timeZone } = useUserTimeZone();

  return useMemo(() => ({
    currentLocale,
    timeZone,
    formatDateTime(value: Date | number | string, options?: FormatDateTimeOptions) {
      const date = new Date(value);
      if (Number.isNaN(date.getTime())) return String(value);
      return new Intl.DateTimeFormat(currentLocale, { timeZone, ...options }).format(date);
    },
    formatNumber(value: number, options?: FormatNumberOptions) {
      return new Intl.NumberFormat(currentLocale, options).format(value);
    },
  }), [currentLocale, timeZone]);
}
