import {
  spawn,
  type ChildProcess,
  type SpawnOptions,
} from "node:child_process";
import type { Readable } from "node:stream";

export type ManagedProcess = ChildProcess & {
  stderr: Readable;
  stdout: Readable;
};

export type ManagedProcessSpawnOptions = Omit<SpawnOptions, "detached" | "stdio"> & {
  stdio?: "pipe" | ["ignore" | "pipe" | null | undefined, "pipe", "pipe"];
};

export type ManagedProcessStopOptions = {
  forceTimeoutMs?: number;
  gracefulTimeoutMs?: number;
};

type ManagedProcessRecord = {
  groupId: number;
  process: ManagedProcess;
  state: "running" | "stopped" | "stopping";
  stopPromise?: Promise<void>;
};

const DEFAULT_FORCE_TIMEOUT_MS = 1_000;
const DEFAULT_GRACEFUL_TIMEOUT_MS = 5_000;
const PROCESS_GROUP_POLL_INTERVAL_MS = 25;

const processGroups = new Map<number, ManagedProcessRecord>();
const processRecords = new WeakMap<ManagedProcess, ManagedProcessRecord>();

let terminating = false;

function assertSupportedPlatform() {
  if (process.platform === "win32") {
    throw new Error("Managed test processes support macOS and Linux only.");
  }
}

function isProcessGroupAlive(groupId: number) {
  try {
    process.kill(-groupId, 0);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ESRCH") return false;
    if ((error as NodeJS.ErrnoException).code === "EPERM") return true;
    throw error;
  }
}

function signalProcessGroup(groupId: number, signal: NodeJS.Signals) {
  try {
    process.kill(-groupId, signal);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ESRCH") return;
    throw error;
  }
}

function delay(ms: number) {
  return new Promise<void>((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

async function waitForProcessGroupExit(groupId: number, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;

  while (isProcessGroupAlive(groupId)) {
    const remainingMs = deadline - Date.now();
    if (remainingMs <= 0) return false;
    await delay(Math.min(PROCESS_GROUP_POLL_INTERVAL_MS, remainingMs));
  }

  return true;
}

function closeProcessPipes(managedProcess: ManagedProcess) {
  managedProcess.stdin?.destroy();
  managedProcess.stdout.destroy();
  managedProcess.stderr.destroy();
}

function markProcessStopped(record: ManagedProcessRecord) {
  record.state = "stopped";
  processGroups.delete(record.groupId);
  closeProcessPipes(record.process);
}

function registerProcess(managedProcess: ManagedProcess) {
  const groupId = managedProcess.pid;
  if (!groupId) return;

  const record: ManagedProcessRecord = {
    groupId,
    process: managedProcess,
    state: "running",
  };
  processGroups.set(groupId, record);
  processRecords.set(managedProcess, record);

  managedProcess.once("exit", () => {
    setImmediate(() => {
      if (record.state === "running" && !isProcessGroupAlive(groupId)) {
        markProcessStopped(record);
      }
    });
  });
}

export function spawnManagedProcess(
  command: string,
  args: readonly string[],
  options: ManagedProcessSpawnOptions = {},
) {
  assertSupportedPlatform();

  const managedProcess = spawn(command, args, {
    ...options,
    detached: true,
  }) as ManagedProcess;

  if (managedProcess.pid) {
    registerProcess(managedProcess);
  } else {
    managedProcess.once("spawn", () => registerProcess(managedProcess));
  }

  return managedProcess;
}

export async function stopManagedProcess(
  managedProcess: ManagedProcess | null | undefined,
  options: ManagedProcessStopOptions = {},
) {
  if (!managedProcess) return;

  const record = processRecords.get(managedProcess);
  if (!record || record.state === "stopped") {
    closeProcessPipes(managedProcess);
    return;
  }
  if (record.stopPromise) return record.stopPromise;

  const gracefulTimeoutMs = options.gracefulTimeoutMs ?? DEFAULT_GRACEFUL_TIMEOUT_MS;
  const forceTimeoutMs = options.forceTimeoutMs ?? DEFAULT_FORCE_TIMEOUT_MS;

  record.state = "stopping";
  record.stopPromise = (async () => {
    try {
      if (!isProcessGroupAlive(record.groupId)) {
        markProcessStopped(record);
        return;
      }

      signalProcessGroup(record.groupId, "SIGTERM");
      if (await waitForProcessGroupExit(record.groupId, gracefulTimeoutMs)) {
        markProcessStopped(record);
        return;
      }

      signalProcessGroup(record.groupId, "SIGKILL");
      if (await waitForProcessGroupExit(record.groupId, forceTimeoutMs)) {
        markProcessStopped(record);
        return;
      }

      throw new Error(
        `Managed process group ${record.groupId} remained alive after SIGTERM and SIGKILL.`,
      );
    } catch (error) {
      record.state = "running";
      record.stopPromise = undefined;
      closeProcessPipes(record.process);
      throw error;
    }
  })();

  return record.stopPromise;
}

export async function stopManagedProcesses(
  managedProcesses: Iterable<ManagedProcess | null | undefined>,
  options: ManagedProcessStopOptions = {},
) {
  const uniqueProcesses = [
    ...new Set([...managedProcesses].filter((item): item is ManagedProcess => Boolean(item))),
  ];
  const results = await Promise.allSettled(
    uniqueProcesses.map((managedProcess) => stopManagedProcess(managedProcess, options)),
  );
  const errors = results.flatMap((result) =>
    result.status === "rejected" ? [result.reason] : [],
  );

  if (errors.length > 0) {
    throw new AggregateError(errors, "Failed to stop one or more managed test processes.");
  }
}

function stopRegisteredProcessGroupsSync() {
  for (const record of processGroups.values()) {
    try {
      signalProcessGroup(record.groupId, "SIGKILL");
    } catch {
      // The worker is already exiting; continue terminating every registered group.
    }
    record.state = "stopped";
    closeProcessPipes(record.process);
  }
  processGroups.clear();
}

type TerminationSignal = "SIGHUP" | "SIGINT" | "SIGTERM";

function handleTerminationSignal(signal: TerminationSignal) {
  if (terminating) return;
  terminating = true;
  stopRegisteredProcessGroupsSync();
  process.removeListener(signal, terminationHandlers[signal]);
  process.kill(process.pid, signal);
}

const terminationHandlers = {
  SIGHUP: () => handleTerminationSignal("SIGHUP"),
  SIGINT: () => handleTerminationSignal("SIGINT"),
  SIGTERM: () => handleTerminationSignal("SIGTERM"),
} satisfies Record<TerminationSignal, () => void>;

process.once("exit", stopRegisteredProcessGroupsSync);
process.on("SIGHUP", terminationHandlers.SIGHUP);
process.on("SIGINT", terminationHandlers.SIGINT);
process.on("SIGTERM", terminationHandlers.SIGTERM);
