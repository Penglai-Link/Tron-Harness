export type LogLevel = "Trace" | "Debug" | "Info" | "Warn" | "Error";
export interface ApplicationLog {
  logId: string;
  timestamp: number;
  level: LogLevel;
  target: string;
  message: string;
  traceId: string | null;
  spanId: string | null;
  attributes: Record<string, string>;
}
export interface LogQuery {
  workspaceCode?: string;
  date: string;
  timeZone: string;
  keyword: string;
  level: LogLevel | "";
  traceId: string;
  spanId: string;
  current: number;
  size: number;
  beforeLogId?: string | null;
  afterLogId?: string | null;
}
export interface ApplicationLogPage {
  date: string;
  timeZone: string;
  current: number;
  size: number;
  total: number;
  records: ApplicationLog[];
}
