import { Badge } from "@/components/ui/badge";

import { getModelSourceTone } from "@/domain/model/constants/modelPageOptions";
import type { ModelSourceKind } from "@/domain/model/model";

export function ModelSourceBadge({
  sourceKind,
  label,
}: {
  sourceKind: ModelSourceKind;
  label: string;
}) {
  return <Badge variant={getModelSourceTone(sourceKind)}>{label}</Badge>;
}
