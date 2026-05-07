export interface AgentMessage {
  id: string;
  role: "user" | "agent";
  body: string;
}
