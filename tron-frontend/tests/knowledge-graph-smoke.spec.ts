import {
  expect,
  test,
  type Page,
} from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

test.describe.configure({ timeout: 45_000 });

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-4000);
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();

    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        rejectPort(new Error("Failed to reserve a frontend test port."));
        return;
      }

      const { port } = address;
      server.close(() => resolvePort(port));
    });
  });
}

async function waitForFrontend(url: string) {
  const deadline = Date.now() + 30_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${frontendLogs}`);
    }

    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`Frontend dev server did not become ready: ${String(lastError)}\n${frontendLogs}`);
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;

  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_KNOWLEDGE_GRAPH_DATA_SOURCE: "mock",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/knowledge-graph`);
});

test.afterAll(async () => {
  if (!frontendProcess) return;
  frontendProcess.kill("SIGTERM");
  await new Promise<void>((resolveProcess) => {
    frontendProcess?.once("exit", () => resolveProcess());
    setTimeout(() => resolveProcess(), 5_000);
  });
});

test("renders backend-shaped graph mock data and updates inspector from result selection", async ({ page }: { page: Page }) => {
  await page.goto(`${frontendBaseUrl}/knowledge-graph`);

  await expect(page.getByRole("heading", { name: "Knowledge Graph" })).toBeVisible();
  await expect(page.getByText("Nodes").first()).toBeVisible();
  await expect(page.getByText("Namespaces").first()).toBeVisible();
  await expect(page.getByText("RAG_REVIEWER").first()).toBeVisible();
  await expect(page.getByText("REQUIRES_TOOL").first()).toBeVisible();
  await expect(page.getByText("namespace").first()).toBeVisible();
  await expect(page.getByText("rag_skill").first()).toBeVisible();
  await expect(page.getByLabel("Semantic focus graph canvas")).toBeVisible();
  await expect(page.getByText("Primary relationships").first()).toBeVisible();

  await page.getByPlaceholder("Search node_code, label, property...").fill("RagTool");
  await expect(page.getByRole("button", { name: /GIT RagTool/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /RAG_REVIEWER RagSkill/i })).toHaveCount(0);

  await page.getByRole("button", { name: /GIT RagTool/i }).click();
  await expect(page.getByLabel("Selected GIT graph node")).toBeVisible();
  await expect(page.getByRole("heading", { name: "GIT" })).toBeVisible();
  await expect(page.getByText("provider").first()).toBeVisible();
  await expect(page.getByText("git").first()).toBeVisible();
  await expect(page.getByText("priority").first()).toBeVisible();
});
