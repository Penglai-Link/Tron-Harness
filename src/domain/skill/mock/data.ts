import type {
  SkillSchemaPreview,
  CapabilityRecord,
} from "@/domain/skill/model/model";

export function getSkillSchemaPreview(): SkillSchemaPreview {
  return {
    name: "get_weather",
    fields: [
      { name: "location", type: "string", required: true },
      { name: "unit", type: "enum", required: false },
    ],
  };
}

const capabilities: CapabilityRecord[] = [
  {
    id: "python",
    mode: "skills",
    name: "Python Code Interpreter",
    description: "Executes Python code in an isolated sandbox environment.",
    provider: "Local Runtime",
    status: "Connected & Healthy",
    enabled: true,
    latency: "42ms",
    schema: '{\n  "name": "execute_python",\n  "parameters": { "code": "string", "timeout": 30 }\n}',
    activity: ["Script execution completed", "Sandbox image refreshed"],
  },
  {
    id: "postgres",
    mode: "mcp",
    name: "Postgres Connector",
    description: "Read-only connection to the primary analytics database.",
    provider: "MCP Server",
    status: "Connected",
    enabled: true,
    latency: "68ms",
    schema: '{\n  "name": "query_postgres",\n  "parameters": { "sql": "string" }\n}',
    activity: ["Schema introspection completed", "Read policy verified"],
  },
  {
    id: "sendgrid",
    mode: "skills",
    name: "SendGrid Mailer",
    description: "Dispatches transactional emails to users via template.",
    provider: "SendGrid",
    status: "Disabled",
    enabled: false,
    latency: "n/a",
    schema: '{\n  "name": "send_email",\n  "parameters": { "template": "string", "to": "string" }\n}',
    activity: ["Disabled by workspace policy"],
  },
  {
    id: "stripe",
    mode: "mcp",
    name: "Stripe Billing API",
    description: "Manage customer subscriptions and invoice retrieval.",
    provider: "MCP Server",
    status: "Connected",
    enabled: true,
    latency: "91ms",
    schema: '{\n  "name": "lookup_invoice",\n  "parameters": { "customer_id": "string" }\n}',
    activity: ["Invoice retrieval completed", "OAuth token rotated"],
  },
];

export function listCapabilities() {
  return capabilities;
}
