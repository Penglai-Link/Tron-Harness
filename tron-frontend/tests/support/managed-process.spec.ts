import { expect, test } from "@playwright/test";
import { createConnection, createServer } from "node:net";

import {
  spawnManagedProcess,
  stopManagedProcess,
} from "./managed-process";

function delay(ms: number) {
  return new Promise<void>((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();
    server.once("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        rejectPort(new Error("Failed to reserve a managed-process test port."));
        return;
      }
      server.close(() => resolvePort(address.port));
    });
  });
}

function canConnect(port: number) {
  return new Promise<boolean>((resolveConnection) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    socket.once("connect", () => {
      socket.destroy();
      resolveConnection(true);
    });
    socket.once("error", () => resolveConnection(false));
  });
}

async function waitForPortState(port: number, expectedOpen: boolean) {
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    if ((await canConnect(port)) === expectedOpen) return;
    await delay(25);
  }
  throw new Error(`Port ${port} did not become ${expectedOpen ? "open" : "closed"}.`);
}

function isProcessGroupAlive(groupId: number) {
  try {
    process.kill(-groupId, 0);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ESRCH") return false;
    throw error;
  }
}

test("stops descendants and releases their port after the managed parent exits", async () => {
  const port = await reservePort();
  const serverScript = [
    "const { createServer } = require('node:net');",
    "const server = createServer((socket) => socket.end('ok'));",
    "server.listen(Number(process.argv[1]), '127.0.0.1');",
    "setInterval(() => {}, 1000);",
  ].join("");
  const parentScript = [
    "const { spawn } = require('node:child_process');",
    `const child = spawn(process.execPath, ['-e', ${JSON.stringify(serverScript)}, process.argv[1]], `,
    "{ stdio: ['ignore', 'inherit', 'inherit'] });",
    "child.unref();",
  ].join("");

  const managedProcess = spawnManagedProcess(
    process.execPath,
    ["-e", parentScript, String(port)],
    { stdio: "pipe" },
  );
  const groupId = managedProcess.pid;
  if (!groupId) throw new Error("Managed parent process did not receive a process group id.");

  try {
    await waitForPortState(port, true);
    await new Promise<void>((resolveExit, rejectExit) => {
      if (managedProcess.exitCode !== null) {
        resolveExit();
        return;
      }
      managedProcess.once("exit", () => resolveExit());
      managedProcess.once("error", rejectExit);
    });
    expect(isProcessGroupAlive(groupId)).toBe(true);

    await stopManagedProcess(managedProcess);

    await waitForPortState(port, false);
    expect(isProcessGroupAlive(groupId)).toBe(false);
  } finally {
    await stopManagedProcess(managedProcess);
  }
});

test("escalates to SIGKILL when the process group ignores SIGTERM", async () => {
  const port = await reservePort();
  const serverScript = [
    "const { createServer } = require('node:net');",
    "process.on('SIGTERM', () => console.log('SIGTERM_IGNORED'));",
    "createServer((socket) => socket.end('ok')).listen(Number(process.argv[1]), '127.0.0.1');",
    "setInterval(() => {}, 1000);",
  ].join("");
  const managedProcess = spawnManagedProcess(
    process.execPath,
    ["-e", serverScript, String(port)],
    { stdio: "pipe" },
  );
  const groupId = managedProcess.pid;
  if (!groupId) throw new Error("Managed process did not receive a process group id.");
  let output = "";
  managedProcess.stdout.on("data", (chunk) => {
    output += chunk.toString();
  });

  try {
    await waitForPortState(port, true);
    await stopManagedProcess(managedProcess, {
      forceTimeoutMs: 1_000,
      gracefulTimeoutMs: 100,
    });

    await waitForPortState(port, false);
    expect(output).toContain("SIGTERM_IGNORED");
    expect(isProcessGroupAlive(groupId)).toBe(false);
  } finally {
    await stopManagedProcess(managedProcess, {
      forceTimeoutMs: 1_000,
      gracefulTimeoutMs: 100,
    });
  }
});

test("treats missing, exited, and repeatedly stopped processes as no-ops", async () => {
  await stopManagedProcess(undefined);

  const managedProcess = spawnManagedProcess(process.execPath, ["-e", ""], {
    stdio: "pipe",
  });
  await new Promise<void>((resolveExit, rejectExit) => {
    managedProcess.once("exit", () => resolveExit());
    managedProcess.once("error", rejectExit);
  });

  await stopManagedProcess(managedProcess);
  await stopManagedProcess(managedProcess);
});
