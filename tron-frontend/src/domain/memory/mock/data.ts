import type { MemoryFragment } from "@/domain/memory/model/MemoryModel";

export function getMemoryFragments(): MemoryFragment[] {
  return [
    {
      id: "mem-1",
      content: "User prefers terse, direct responses without conversational filler.",
      namespace: "UserPrefs",
      entity: "usr_982x",
      source: "Core Workspace",
      confidence: 0.98,
      created: "2023-10-27",
      inheritance: "Local",
    },
    {
      id: "mem-2",
      content: "System default tone set to professional and helpful.",
      namespace: "SysConfig",
      entity: "global_v1",
      source: "Agent Hub",
      confidence: 1,
      created: "2023-01-15",
      inheritance: "Inherited",
    },
    {
      id: "mem-3",
      content: "Project Alpha requires strict adherence to security protocols outlined in doc-4A.",
      namespace: "ContextDocs",
      entity: "proj_alpha",
      source: "Data Extraction App",
      confidence: 0.92,
      created: "2023-10-20",
      inheritance: "Override",
    },
    {
      id: "mem-4",
      content: "User frequently asks about deployment pipeline statuses on Fridays.",
      namespace: "BehavioralInsights",
      entity: "usr_982x",
      source: "Core Workspace",
      confidence: 0.86,
      created: "2023-10-26",
      inheritance: "Local",
    },
    {
      id: "mem-5",
      content: "Standard error handling procedure for database connection timeouts.",
      namespace: "SOPs",
      entity: "eng_team",
      source: "Agent Hub",
      confidence: 0.99,
      created: "2023-08-12",
      inheritance: "Inherited",
    },
  ];
}
