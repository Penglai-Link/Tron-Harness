export const INTERNAL_AGENT_REGISTRY_CHANGED_EVENT = "tron:internal-agent-registry-changed";

export function notifyInternalAgentRegistryChanged() {
  window.dispatchEvent(new Event(INTERNAL_AGENT_REGISTRY_CHANGED_EVENT));
}
