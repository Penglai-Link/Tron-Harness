import type { StatusTone } from "../constants/StatusTone";

export interface QuickReference {
  tips: string[];
  variables: string[];
}

export interface TaskDetailData {
  id: string;
  title: string;
  status: string;
  priority: string;
  progress: number;
  agent: string;
  description: string;
  requirements: string[];
  resources: string[];
  timeline: { time: string; message: string; tone?: StatusTone }[];
  metadata: Record<string, string>;
  subtasks: { id: string; title: string; status: string; progress: number; done: boolean }[];
}

export interface KanbanTask {
  id: string;
  title: string;
  priority: "LOW" | "MED" | "HIGH";
  progress: number;
  owner: string;
  signal: string;
}

export interface KanbanColumn {
  id: string;
  title: string;
  accent: string;
  tasks: KanbanTask[];
}
