import { Badge } from "@/shared/ui/Badge";

import { getModelSourceTone } from "@/domain/model/constants/modelPageOptions";
import type { ModelSourceKind } from "@/domain/model/model";

export function ModelSourceBadge({
  sourceKind,
  label,
}: {
  sourceKind: ModelSourceKind;
  label: string;
}) {
  return <Badge tone={getModelSourceTone(sourceKind)}>{label}</Badge>;
}
