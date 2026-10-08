import type { PromptRecord } from "./PromptModel";

export function localizedPromptContent(record: PromptRecord, locale: string) {
  const translation = record.translations?.find((item) => item.locale === locale && item.status === "PUBLISHED");
  return {
    displayName: translation?.displayName.trim() || record.displayName,
    description: translation?.description ?? record.description,
    promptBody: translation?.promptBody ?? record.promptBody,
  };
}
