import type { TFunction } from "i18next";

// Stable catalog identities select translations; external metadata stays intact.
export function capabilityDescription(t: TFunction, kind: string, name: string, fallback: string) {
  const key = `${kind}_${name}`.replace(/[^a-zA-Z0-9_-]/g, "_");
  return t(`pages:capabilityManagement.catalog.${key}`, { defaultValue: fallback });
}
