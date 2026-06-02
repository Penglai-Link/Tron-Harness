import {
  expect,
  test,
  type Locator,
  type Page,
} from "@playwright/test";
import {
  existsSync,
  readFileSync,
} from "node:fs";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
const toastSourcePath = resolve(frontendRoot, "src/shared/ui/Toast.tsx");
const redExpectationTimeout = 2500;
const toastAutoDismissTimeout = 3000;

let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-4000);
}

function source(pathFromFrontendRoot: string): string {
  return readFileSync(
    resolve(frontendRoot, pathFromFrontendRoot),
    "utf8",
  );
}

function sourceIfExists(pathFromFrontendRoot: string): string {
  const absolutePath = resolve(frontendRoot, pathFromFrontendRoot);
  return existsSync(absolutePath) ? readFileSync(absolutePath, "utf8") : "";
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

async function stopFrontend() {
  const processToStop = frontendProcess;
  if (!processToStop || processToStop.exitCode !== null) return;

  const exited = new Promise((resolveExit) => {
    processToStop.once("exit", resolveExit);
  });
  processToStop.kill("SIGTERM");
  await Promise.race([exited, delay(2000)]);
  if (processToStop.exitCode === null) {
    processToStop.kill("SIGKILL");
  }
}

async function openSharedShellPreview(page: Page) {
  await page.goto(`${frontendBaseUrl}/shared-shell-preview`);
  await expect(page).toHaveURL(/\/shared-shell-preview$/);
  await expect(page.getByRole("heading", { name: "Workspace console foundation" })).toBeVisible();
}

function scanExportToast(page: Page) {
  return page.locator("[role='status']").filter({ hasText: "Scan export requested" }).first();
}

function toastCloseButton(page: Page) {
  return page.getByRole("button", {
    name: /close toast|dismiss toast|close notification|dismiss notification/i,
  }).first();
}

async function expectToastInTopRightViewport(toast: Locator, page: Page) {
  const box = await toast.boundingBox();
  const viewport = page.viewportSize();

  expect(box, "The toast must have a rendered bounding box.").not.toBeNull();
  expect(viewport, "The browser viewport must be available for placement checks.").not.toBeNull();
  expect(box!.x, "Toast should float on the right side of the viewport.").toBeGreaterThan(viewport!.width * 0.55);
  expect(box!.y, "Toast should float near the top of the viewport.").toBeLessThan(140);
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        CI: "1",
        BROWSER: "none",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_ADMIN_DATA_SOURCE: "mock",
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(frontendBaseUrl);
});

test.afterAll(async () => {
  await stopFrontend();
});

test("TDD-FEEDBACK-01: shared toast provider, hook, and public export are wired", async () => {
  const sharedExports = source("src/shared/index.ts");
  const appSource = source("src/app/App.tsx");
  const toastSource = sourceIfExists("src/shared/ui/Toast.tsx");

  expect.soft(
    existsSync(toastSourcePath),
    "src/shared/ui/Toast.tsx should define the shared ToastProvider/useToast API.",
  ).toBe(true);
  expect.soft(toastSource, "ToastProvider should be implemented by the shared UI primitive.").toContain("ToastProvider");
  expect.soft(toastSource, "useToast should be implemented by the shared UI primitive.").toContain("useToast");
  expect.soft(toastSource, "The shared toast API should expose a generic show operation.").toContain("show");
  expect.soft(toastSource, "The shared toast API should expose a success convenience operation.").toContain("success");
  expect.soft(toastSource, "The shared toast API should expose an error convenience operation.").toContain("error");
  expect.soft(toastSource, "The shared toast API should expose a warning convenience operation.").toContain("warning");
  expect.soft(toastSource, "The shared toast API should expose an info convenience operation.").toContain("info");
  expect.soft(toastSource, "The shared toast API should expose dismiss for manual removal.").toContain("dismiss");
  expect.soft(sharedExports, "The public shared boundary should export the toast primitive.").toContain("./ui/Toast");
  expect.soft(appSource, "The app root should mount ToastProvider around routed pages.").toContain("ToastProvider");
});

test("TDD-FEEDBACK-02: Export Scan shows a shared top-right status toast", async ({ page }) => {
  await openSharedShellPreview(page);

  await page.getByRole("button", { name: "Export Scan" }).click();

  const toast = scanExportToast(page);
  await expect(toast, "Export Scan should render a shared toast with role='status'.").toBeVisible({
    timeout: redExpectationTimeout,
  });
  await expectToastInTopRightViewport(toast, page);
});

test("TDD-FEEDBACK-03: shared toast auto-dismisses and supports manual dismissal", async ({ page }) => {
  await openSharedShellPreview(page);

  await page.getByRole("button", { name: "Export Scan" }).click();
  const autoDismissToast = scanExportToast(page);
  let renderedForAutoDismiss = true;
  try {
    await expect(autoDismissToast, "Auto-dismiss requires a rendered toast before the timeout.").toBeVisible({
      timeout: redExpectationTimeout,
    });
  } catch (error) {
    renderedForAutoDismiss = false;
    expect.soft(false, `Auto-dismiss behavior is missing because no shared toast rendered: ${String(error)}`).toBe(true);
  }

  if (renderedForAutoDismiss) {
    await expect(autoDismissToast, "The shared toast should disappear without operator cleanup.").toBeHidden({
      timeout: toastAutoDismissTimeout,
    });
  }

  await page.getByRole("button", { name: "Export Scan" }).click();
  const dismissibleToast = scanExportToast(page);
  let renderedForManualDismiss = true;
  try {
    await expect(dismissibleToast, "Manual dismissal requires a rendered toast.").toBeVisible({
      timeout: redExpectationTimeout,
    });
  } catch (error) {
    renderedForManualDismiss = false;
    expect.soft(false, `Manual dismissal behavior is missing because no shared toast rendered: ${String(error)}`).toBe(true);
  }

  if (renderedForManualDismiss) {
    const closeButton = toastCloseButton(page);
    await expect(closeButton, "The shared toast should expose a named close/dismiss control.").toBeVisible({
      timeout: redExpectationTimeout,
    });
    await closeButton.click();
    await expect(dismissibleToast, "The shared toast should disappear after manual dismissal.").toBeHidden({
      timeout: redExpectationTimeout,
    });
  }
});
