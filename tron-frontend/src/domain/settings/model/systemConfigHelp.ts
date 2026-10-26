import type { TFunction } from "i18next";

// Profile names are user-defined. Normalize only their name segment, retaining
// the owning subsystem so identical leaf names never share unrelated help.
export function systemConfigHelpKey(path: string) {
  const scoped = path
    .replace(/^shortTermMemory\.context\.providers\.dataBlocks\..+\./, "shortTermMemory.context.providers.dataBlocks.block.")
    .replace(/^resource\.storage\.profiles\.[^.]+\./, "resource.storage.profiles.profile.")
    .replace(/^httpClients\.(default|modelInference|runtimeControl|documentParsing)\./, "httpClients.profile.");
  return `settingsGeneral.systemConfig.descriptions.${scoped.replaceAll(".", "_")}`;
}

export function systemConfigHelp(path: string, t: TFunction<"pages">) {
  // An unsupported future field must not display a fabricated explanation.
  return t(systemConfigHelpKey(path), { defaultValue: "" });
}
