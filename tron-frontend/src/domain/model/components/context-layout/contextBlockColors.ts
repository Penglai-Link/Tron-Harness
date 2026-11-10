// Stable identities keep the chart and editor aligned when blocks are reordered.
// Use the active skin's categorical chart colors, including monochrome skins.
const chart = (index: number) => `var(--chart-${index})`;
const blend = (first: number, second: number) => `color-mix(in oklab, ${chart(first)}, ${chart(second)})`;
const colors: Record<string, string> = {
  seed_prompt_prefix: chart(1),
  conversation_history_summary: chart(4),
  runtime_identity: blend(2, 4),
  dude_onboarding: blend(3, 4),
  tool_domains: chart(3),
  current_session_history: chart(2),
  agent_directory: blend(1, 4),
  "skills.catalog": chart(5),
  page_context: blend(1, 2),
  application_context: blend(4, 5),
  current_session_working_state: blend(3, 5),
  long_term_memory: blend(3, 4),
  knowledge_base: blend(2, 3),
  dude_environment: blend(1, 3),
  dude_wakeup: blend(1, 5),
};

export function contextBlockColor(id: string): string {
  if (colors[id]) return colors[id];
  const hash = [...id].reduce((value, character) => (value * 31 + character.charCodeAt(0)) >>> 0, 0);
  return chart(hash % 5 + 1);
}
