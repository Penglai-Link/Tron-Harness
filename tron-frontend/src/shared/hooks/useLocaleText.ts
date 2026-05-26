import { useCallback } from "react";
import { useTranslation } from "react-i18next";

type LocaleTextOptions = Record<string, unknown>;

export function useLocaleText(namespace = "pages") {
  const { t } = useTranslation(namespace);

  return useCallback(
    (defaultValue: string, options?: LocaleTextOptions) =>
      t(defaultValue, {
        ...(options ?? {}),
        defaultValue,
        keySeparator: false,
        returnObjects: false,
      } as never) as unknown as string,
    [t],
  );
}
