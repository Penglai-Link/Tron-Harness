export interface DashboardMetric {
  label: string;
  value: string;
  trend: string;
}

export interface DashboardActivity {
  title: string;
  meta: string;
  tone: "neutral" | "warning" | "success";
}

export interface ImportApplicationDefaults {
  source: "github" | "gitlab" | "local";
  repositoryUrl: string;
  accessToken: string;
  localPath: string;
  appName: string;
  tags: string[];
  branch: string;
}

export interface GraphEntity {
  id: string;
  label: string;
  type: string;
  properties: Record<string, string>;
  relationships: Array<{ label: string; target: string }>;
  x: number;
  y: number;
}

export interface MemoryFragment {
  id: string;
  content: string;
  namespace: string;
  entity: string;
  source: string;
  confidence: number;
  created: string;
  inheritance: "Local" | "Inherited" | "Override";
}

export interface ModelRegistryItem {
  id: string;
  name: string;
  purpose: string;
  vendor: "OpenAI" | "Anthropic" | "Google";
  context: string;
  inheritance: string;
  status: "Enabled" | "Disabled";
  price: string;
  maxOutput: string;
  trainingData: string;
  speed: string;
  benchmarks: Array<{ label: string; score: number }>;
}

export interface AgentMessage {
  id: string;
  role: "user" | "agent";
  body: string;
}

export function getDashboardData() {
  return {
    metrics: [
      { label: "System Health", value: "99.8%", trend: "+0.2% from last week" },
      { label: "Active Agents", value: "142", trend: "Across 12 clusters" },
      { label: "Running Tasks", value: "3,845", trend: "75% capacity, 2.1k queued" },
      { label: "Models", value: "28", trend: "18 inherited, 10 local" },
      { label: "MCP Servers", value: "16", trend: "12 healthy, 4 monitored" },
      { label: "Skills", value: "84", trend: "67 inherited, 17 custom" },
      { label: "Apps", value: "37", trend: "29 active, 8 in review" },
      { label: "Memory Items", value: "128K", trend: "Across graph and vectors" },
    ] satisfies DashboardMetric[],
    activities: [
      { title: "Skill Refund_Processing deployed", meta: "Just now - admin@system", tone: "success" },
      { title: "High latency detected in Model Router", meta: "2m ago - System Alert", tone: "warning" },
      { title: "Inheritance policy updated (Global)", meta: "15m ago - sec-ops@system", tone: "neutral" },
      { title: "New agent cluster provisioned", meta: "1h ago - auto-scaler", tone: "neutral" },
      { title: "Vector DB sync completed", meta: "3h ago - daily-job", tone: "success" },
    ] satisfies DashboardActivity[],
  };
}

export function getImportApplicationDefaults(source: ImportApplicationDefaults["source"]): ImportApplicationDefaults {
  return {
    source,
    repositoryUrl: source === "github" ? "https://github.com/acme/refund-agent" : "",
    accessToken: "",
    localPath: source === "local" ? "/Users/username/projects/my-app" : "/Users/username/Projects",
    appName: source === "local" ? "local-agent-workbench" : "refund-agent",
    tags: ["Frontend", "React"],
    branch: "main",
  };
}

export function getGraphData(domain = "customer-support") {
  const domainLabel = domain === "supply-chain" ? "Supply Chain" : domain === "hr" ? "HR" : "Customer Support";
  const entities: GraphEntity[] = [
    {
      id: "customer-0192",
      label: domain === "hr" ? "Employee_0192" : "Customer_0192",
      type: domain === "hr" ? "Employee Profile" : "User Profile",
      x: 350,
      y: 250,
      properties: { status: "Active", created_at: "2023-10-24T08:12:00Z", ltv_segment: "High Value", domain: domainLabel },
      relationships: [
        { label: domain === "supply-chain" ? "PLACED_ORDER" : "MADE_PURCHASE", target: "Order_TXN_55" },
        { label: domain === "hr" ? "OWNS_DEVICE" : "MADE_PURCHASE", target: "Order_TXN_56" },
      ],
    },
    {
      id: "order-55",
      label: "Order_TXN_55",
      type: "Transaction",
      x: 500,
      y: 400,
      properties: { status: "Fulfilled", created_at: "2023-10-25T11:09:00Z", total: "$284.10", domain: domainLabel },
      relationships: [{ label: "CONTAINS", target: "Product_SKU_A1" }],
    },
    {
      id: "sku-a1",
      label: "Product_SKU_A1",
      type: "Inventory Item",
      x: 650,
      y: 300,
      properties: { status: "In Stock", created_at: "2023-09-12T03:20:00Z", tier: "Priority", domain: domainLabel },
      relationships: [],
    },
    {
      id: "order-56",
      label: "Order_TXN_56",
      type: "Transaction",
      x: 200,
      y: 400,
      properties: { status: "Review", created_at: "2023-10-26T15:40:00Z", total: "$92.00", domain: domainLabel },
      relationships: [],
    },
  ];

  return { domainLabel, entities };
}

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

export function getModels(): ModelRegistryItem[] {
  return [
    {
      id: "gpt-4-turbo",
      name: "gpt-4-turbo",
      purpose: "Default Reasoning",
      vendor: "OpenAI",
      context: "128k tokens",
      inheritance: "Global",
      status: "Enabled",
      price: "$10.00 / 1M Input",
      maxOutput: "4,096 tokens",
      trainingData: "Dec 2023",
      speed: "~42 tk/s",
      benchmarks: [
        { label: "MMLU (Reasoning)", score: 88.7 },
        { label: "HumanEval (Coding)", score: 87.1 },
        { label: "GSM8K (Math)", score: 95.3 },
      ],
    },
    {
      id: "claude-3-opus",
      name: "claude-3-opus-20240229",
      purpose: "Complex Tasks",
      vendor: "Anthropic",
      context: "200k tokens",
      inheritance: "Workspace A",
      status: "Enabled",
      price: "$15.00 / 1M Input",
      maxOutput: "4,096 tokens",
      trainingData: "Aug 2023",
      speed: "~31 tk/s",
      benchmarks: [
        { label: "MMLU (Reasoning)", score: 86.8 },
        { label: "HumanEval (Coding)", score: 84.9 },
        { label: "GSM8K (Math)", score: 94.2 },
      ],
    },
    {
      id: "gpt-3.5-turbo",
      name: "gpt-3.5-turbo",
      purpose: "Legacy Fast",
      vendor: "OpenAI",
      context: "16k tokens",
      inheritance: "Global",
      status: "Disabled",
      price: "$0.50 / 1M Input",
      maxOutput: "2,048 tokens",
      trainingData: "Sep 2021",
      speed: "~91 tk/s",
      benchmarks: [
        { label: "MMLU (Reasoning)", score: 70.1 },
        { label: "HumanEval (Coding)", score: 48.1 },
        { label: "GSM8K (Math)", score: 57.1 },
      ],
    },
  ];
}

export function getMainAgentMessages(): AgentMessage[] {
  return [
    { id: "m1", role: "user", body: "Assess the current load on the primary clusters and re-allocate resources if we are nearing the 80% threshold." },
    {
      id: "m2",
      role: "agent",
      body: "us-east-1-alpha is currently at 84% CPU utilization. I am preparing a scale-up sequence to add 3 standard nodes.",
    },
  ];
}
