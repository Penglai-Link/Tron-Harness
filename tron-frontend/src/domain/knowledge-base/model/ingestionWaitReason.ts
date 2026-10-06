import type { TFunction } from "i18next";
import type { IngestionWaitReason } from "./KnowledgeLibraryModel";

export function formatIngestionWaitReason(reason: IngestionWaitReason, t: TFunction<"pages">): string {
  const params = { ...reason.params };
  if (typeof params.tier === "string") params.tier = t(`knowledgeLibrary.parsingTiers.${params.tier}`);
  if (params.actual === null) params.actual = t("knowledgeLibrary.unknownValue");
  const details = reason.details?.map((detail) => formatIngestionWaitReason(detail, t)).join(t("knowledgeLibrary.queueReasonSeparator"));
  const message = t(`knowledgeLibrary.queueReasons.${reason.code}`, {
    ...params, details: details ?? "", defaultValue: t("knowledgeLibrary.queueReasons.UNKNOWN"),
  });
  return params.endpointId && reason.code !== "ORIGINAL_NODE_UNAVAILABLE"
    ? t("knowledgeLibrary.queueReasonNode", { node: params.endpointId, reason: message }) : message;
}
