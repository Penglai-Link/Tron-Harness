import type { TFunction } from "i18next";

// Stable catalog identities select translations; external metadata stays intact.
export function capabilityDescription(t: TFunction, kind: string, name: string, fallback: string) {
  // Tool text is already resolved by its owning plugin and Framework. Never
  // look up a host translation for it.
  if (kind === "Tool") return fallback;
  const key = `${kind}_${name}`.replace(/[^a-zA-Z0-9_-]/g, "_");
  return t(`pages:capabilityManagement.catalog.${key}`, { defaultValue: fallback });
}

/** Keep catalog summaries short; detail pages show the pack's feature list. */
export function capabilityPackDetails(t: TFunction, id: string, fallback: string) {
  const key = id.replace(/[^a-zA-Z0-9_-]/g, "_");
  return t(`pages:capabilityManagement.packDetails.${key}`, {
    defaultValue: capabilityDescription(t, "Pack", id, fallback),
  });
}
