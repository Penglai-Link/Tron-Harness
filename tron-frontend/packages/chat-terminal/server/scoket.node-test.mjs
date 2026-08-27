import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";

import { TerminalGateway } from "./scoket.js";
import {
  minifiedIntegrationScript,
  prepareShell,
  shellKind,
  sshBootstrap,
  subshellBootstrap,
} from "./shellIntegration.js";

test("a connection reports the session descriptor the client needs", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);

  assert.equal(harness.terminals.length, 1);
  assert.deepEqual(client.events, [
    [
      "terminal:session",
      {
        currentDirectory: "/workspace",
        shell: "zsh",
        subshellBootstrap: "eval bootstrap",
        supportsBlocks: true,
      },
    ],
  ]);
});

test("a shell without hooks still runs, reported as degraded", () => {
  const harness = createGatewayHarness({
    prepareShell: () => ({
      args: ["-i"],
      command: "/bin/sh",
      env: {},
      kind: "sh",
      supportsBlocks: false,
    }),
  });
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);

  const [, session] = client.events[0];
  assert.equal(session.shell, "sh");
  assert.equal(session.supportsBlocks, false);
  assert.equal(harness.terminals.length, 1);
});

test("every PTY byte is streamed back unframed", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];

  // Prompt markers, prompt text and output all flow through untouched: the
  // client recovers command boundaries from the markers itself.
  terminal.emitData("\x1b]133;A\x07~/project $ ");
  terminal.emitData("total 0\r\n");

  assert.deepEqual(outputs(client), [
    "\x1b]133;A\x07~/project $ ",
    "total 0\r\n",
  ]);
});

test("keystrokes reach the PTY verbatim", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];

  harness.gateway.writeInput(client, { data: "ls -la" });
  harness.gateway.writeInput(client, { data: "\r" });
  harness.gateway.writeInput(client, { data: "" });
  harness.gateway.writeInput(client, { data: "x".repeat(64 * 1024 + 1) });

  assert.deepEqual(terminal.writes, ["ls -la", "\r"]);
});

test("clearBuffer suppresses the readline clear echo", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];
  harness.gateway.writeInput(client, { data: "", clearBuffer: true });
  terminal.emitData("^C\r\n~/project $ ");

  assert.deepEqual(terminal.writes, ["\x03"]);
  assert.deepEqual(outputs(client), []);
});

test("an SSH submission reaches the shell verbatim", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];
  harness.gateway.writeInput(client, {
    data: "\r",
    inputLine: "ssh build@host",
  });

  assert.deepEqual(terminal.writes, ["\r"]);
});

test("SIGINT is written into the PTY so the foreground job receives it", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];

  harness.gateway.sendSignal(client, { signal: "SIGINT" });
  harness.gateway.sendSignal(client, { signal: "SIGWINCH" });

  // Reaches a job running inside `ssh`, which a signal to the child would not.
  assert.deepEqual(terminal.writes, ["\x03"]);
  assert.equal(terminal.killCalls, 0);
});

test("SIGKILL kills the shell instead of typing into it", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];

  harness.gateway.sendSignal(client, { signal: "SIGKILL" });

  assert.equal(terminal.killCalls, 1);
  assert.deepEqual(terminal.writes, []);
});

test("resize is forwarded once the dimensions are sane", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];

  harness.gateway.resizeTerminal(client, { cols: 120, rows: 50 });
  harness.gateway.resizeTerminal(client, { cols: 0, rows: 50 });
  harness.gateway.resizeTerminal(client, { cols: 120.5, rows: 50 });
  harness.gateway.resizeTerminal(client, { cols: 5_000, rows: 50 });

  assert.deepEqual(terminal.resizeCalls, [[120, 50]]);
});

test("the shell exiting is reported with its status and ends the session", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];

  terminal.emitExit(0);

  assert.deepEqual(client.events.at(-1), ["terminal:exit", { exitCode: 0 }]);
  assert.equal(harness.gateway.sessionsByClient.has("client-a"), false);
  // A shell that exited on its own must not be killed again.
  assert.equal(terminal.killCalls, 0);
});

test("a killed shell reports an unknown status", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  harness.terminals[0].emitExit(undefined);

  assert.deepEqual(client.events.at(-1), ["terminal:exit", { exitCode: null }]);
});

test("a spawn failure is reported as a session error", () => {
  const harness = createGatewayHarness({
    spawn: () => {
      throw new Error("终端会话无法启动");
    },
  });
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);

  assert.deepEqual(client.events, [
    ["terminal:session", { error: "终端会话无法启动" }],
  ]);
  assert.equal(harness.cleanups, 1);
});

test("socket sessions are isolated and disconnect cleans only its own PTY", () => {
  const harness = createGatewayHarness();
  const firstClient = createClient("client-a");
  const secondClient = createClient("client-b");

  harness.gateway.handleConnection(firstClient);
  harness.gateway.handleConnection(secondClient);
  assert.equal(harness.terminals.length, 2);

  const [firstTerminal, secondTerminal] = harness.terminals;
  harness.gateway.handleDisconnect(firstClient);
  firstTerminal.emitData("ignored after disconnect");

  assert.equal(firstTerminal.killCalls, 1);
  assert.equal(secondTerminal.killCalls, 0);
  assert.equal(harness.gateway.sessionsByClient.has("client-a"), false);
  assert.equal(harness.gateway.sessionsByClient.has("client-b"), true);
  assert.deepEqual(outputs(firstClient), []);
  // The generated rc files must not be left behind in the temp directory.
  assert.equal(harness.cleanups, 1);
});

test("the integration script reports every semantic marker", () => {
  const script = minifiedIntegrationScript();

  for (const marker of ["133;A", "133;B", "133;C", "133;D;", "633;E;", "7;file://"]) {
    assert.ok(script.includes(marker), `script should report ${marker}`);
  }
  assert.ok(script.includes("ZSH_VERSION"));
  assert.ok(script.includes("BASH_VERSION"));
  // `status` is a read-only alias for `$?` in zsh: assigning to it aborts the
  // hook, and no marker is ever reported.
  assert.ok(
    !/\b(local|typeset)\s+status=/.test(script),
    "hooks must not assign to a shell-reserved name",
  );
  assert.ok(script.includes("return $__tron_status"));
  assert.ok(!script.includes("return $status"));
  assert.ok(!script.split("\n").some((line) => line.trim().startsWith("#")));
});

test("the subshell bootstrap is one decodable line", () => {
  const bootstrap = subshellBootstrap();

  assert.equal(bootstrap.split("\n").length, 1);
  assert.ok(bootstrap.startsWith('eval "$('));
  // BSD base64 has no -d, GNU base64 has no -D; the line must try both.
  assert.ok(bootstrap.includes("base64 -d") && bootstrap.includes("base64 -D"));

  const encoded = bootstrap.match(/'([A-Za-z0-9+/=]+)'/)?.[1];
  assert.ok(encoded, "bootstrap carries a single-quoted payload");
  assert.equal(
    Buffer.from(encoded, "base64").toString("utf8"),
    minifiedIntegrationScript(),
  );
});

test("the SSH bootstrap creates a hooked interactive shell", () => {
  const bootstrap = sshBootstrap();
  const encoded = bootstrap.match(/'([A-Za-z0-9+/=]+)'/)?.[1];
  assert.ok(encoded, "SSH bootstrap carries a single-quoted payload");

  const launcher = Buffer.from(encoded, "base64").toString("utf8");
  assert.ok(!launcher.includes("__TRON_INTEGRATION_BASE64__"));
  assert.ok(launcher.includes("--init-file"));
  assert.ok(launcher.includes("ZDOTDIR"));
  assert.ok(launcher.includes("integration.sh"));
});

test(
  "the SSH helper injects only interactive sessions and clears locale",
  { skip: process.platform === "win32" },
  async () => {
    const shell = prepareShell({ env: {}, shell: "/bin/bash" });
    const directory = await fs.mkdtemp(path.join(os.tmpdir(), "tron-ssh-helper-test-"));
    const fakeSsh = path.join(directory, "fake-ssh");
    await fs.writeFile(
      fakeSsh,
      [
        "#!/bin/sh",
        "for argument do printf 'ARG=%s\\n' \"$argument\"; done",
        "printf 'LANG=%s\\n' \"${LANG-unset}\"",
        "printf 'LC_ALL=%s\\n' \"${LC_ALL-unset}\"",
        "exit \"${TRON_TEST_EXIT:-0}\"",
        "",
      ].join("\n"),
    );
    await fs.chmod(fakeSsh, 0o755);

    const run = (args, extraEnv = {}) => spawnSync(
      "/bin/sh",
      [shell.env.__TRON_SSH_HELPER, ...args],
      {
        encoding: "utf8",
        env: {
          ...process.env,
          ...extraEnv,
          __TRON_REAL_SSH: fakeSsh,
          __TRON_SSH_BOOTSTRAP: "remote bootstrap",
          LANG: "invalid.UTF-8",
          LC_ALL: "invalid.UTF-8",
        },
      },
    );

    try {
      const interactive = run(["-p", "2222", "build@host"]);
      assert.equal(interactive.status, 0, interactive.stderr);
      assert.ok(interactive.stdout.includes("ARG=-p\nARG=2222\nARG=build@host\n"));
      assert.ok(interactive.stdout.includes("ARG=-t\nARG=remote bootstrap\n"));
      assert.ok(interactive.stdout.includes("LANG=unset\nLC_ALL=unset\n"));

      for (const args of [
        ["host", "uptime"],
        ["-T", "host"],
        ["-N", "host"],
        ["-W", "target:22", "host"],
      ]) {
        assert.ok(!run(args).stdout.includes("ARG=remote bootstrap\n"));
      }

      assert.equal(run(["host"], { TRON_TEST_EXIT: "23" }).status, 23);
    } finally {
      shell.cleanup?.();
      await fs.rm(directory, { force: true, recursive: true });
    }
  },
);

test("shell kind is derived from the program name", () => {
  assert.equal(shellKind("/bin/bash"), "bash");
  assert.equal(shellKind("/opt/homebrew/bin/zsh"), "zsh");
  assert.equal(shellKind("/bin/sh"), "sh");
});

test(
  "the generated rc files load user configuration before the hooks",
  { skip: process.platform === "win32" },
  async () => {
    const bash = prepareShell({ env: {}, shell: "/bin/bash" });
    try {
      assert.equal(bash.kind, "bash");
      assert.equal(bash.supportsBlocks, true);
      assert.ok(bash.env.__TRON_SUBSHELL_BOOTSTRAP.includes("base64 -d"));
      const rcPath = bash.args[bash.args.indexOf("--init-file") + 1];
      const rc = await fs.readFile(rcPath, "utf8");
      assert.ok(
        rc.indexOf(".bashrc") < rc.indexOf("integration.sh"),
        "user configuration must load first",
      );
    } finally {
      bash.cleanup?.();
    }

    const zsh = prepareShell({ env: { ZDOTDIR: "/home/user/zsh" }, shell: "/bin/zsh" });
    try {
      assert.equal(zsh.kind, "zsh");
      assert.equal(zsh.env.__TRON_ORIGINAL_ZDOTDIR, "/home/user/zsh");
      const rc = await fs.readFile(path.join(zsh.env.ZDOTDIR, ".zshrc"), "utf8");
      assert.ok(rc.includes("__TRON_ORIGINAL_ZDOTDIR"));
      assert.ok(rc.includes("integration.sh"));
      assert.ok(
        await fs
          .stat(path.join(zsh.env.ZDOTDIR, ".zshenv"))
          .then((stat) => stat.isFile()),
      );
    } finally {
      zsh.cleanup?.();
    }
  },
);

test(
  "a real shell reports prompt markers, a command and its exit code",
  { skip: process.platform === "win32", timeout: 15_000 },
  async () => {
    const directory = await fs.mkdtemp(path.join(os.tmpdir(), "tron-pty-test-"));
    const gateway = new TerminalGateway({ cwd: directory });
    const client = createClient("real-client");

    try {
      gateway.handleConnection(client);
      const [, session] = client.events[0];
      if (!session.supportsBlocks) {
        return; // No bash/zsh on this machine; nothing to assert.
      }

      await waitForOutput(client, (data) => data.includes("\x1b]133;A"));

      // A subshell so the status is reported at the next prompt; `exit 3` would
      // take the shell down before it could report anything.
      gateway.writeInput(client, { data: "printf hello; (exit 3)\r" });

      // The shell frames the command itself, including its status.
      await waitForJoined(client, (output) => /\x1b\]133;D;3/.test(output));
      const framed = joinedOutput(client);
      assert.match(framed, /\x1b\]133;C/);
      assert.ok(framed.includes("hello"), "command output should reach the client");

      gateway.writeInput(client, { data: "exit 7\r" });
      const exit = await waitForEvent(client, ([event]) => event === "terminal:exit");
      assert.equal(exit[1].exitCode, 7);
    } finally {
      gateway.handleDisconnect(client);
      await fs.rm(directory, { force: true, recursive: true });
    }
  },
);

test(
  "a real shell reports its working directory as it changes",
  { skip: process.platform === "win32", timeout: 15_000 },
  async () => {
    const directory = await fs.realpath(
      await fs.mkdtemp(path.join(os.tmpdir(), "tron-cwd-test-")),
    );
    await fs.mkdir(path.join(directory, "nested"));
    const gateway = new TerminalGateway({ cwd: directory });
    const client = createClient("cwd-client");

    try {
      gateway.handleConnection(client);
      const [, session] = client.events[0];
      if (!session.supportsBlocks) {
        return;
      }

      await waitForOutput(client, (data) => data.includes("\x1b]133;A"));
      gateway.writeInput(client, { data: "cd nested\r" });
      await waitForOutput(client, (data) =>
        data.includes(`7;file://`) && data.includes("/nested"),
      );
    } finally {
      gateway.handleDisconnect(client);
      await fs.rm(directory, { force: true, recursive: true });
    }
  },
);

test(
  "a real shell clearBuffer cancels pending input before the next submission",
  { skip: process.platform === "win32", timeout: 15_000 },
  async () => {
    const gateway = new TerminalGateway({ cwd: process.cwd() });
    const client = createClient("clear-input-client");

    try {
      gateway.handleConnection(client);
      await waitForOutput(client, (data) => data.includes("\x1b]133;A"));

      gateway.writeInput(client, { data: "tron_command_that_must_not_run" });
      await delay(20);
      gateway.writeInput(client, { data: "", clearBuffer: true });
      await delay(300);
      gateway.writeInput(client, { data: "printf input-cleared\r" });

      await waitForOutput(client, () => joinedOutput(client).includes("input-cleared"));
      assert.ok(!joinedOutput(client).includes("command not found"));
    } finally {
      gateway.handleDisconnect(client);
    }
  },
);

test(
  "a real shell promotes C locale before editing Chinese input",
  { skip: process.platform === "win32", timeout: 15_000 },
  async () => {
    const gateway = new TerminalGateway({
      cwd: process.cwd(),
      env: { ...process.env, LANG: "C", LC_ALL: "C", LC_CTYPE: "C" },
    });
    const client = createClient("utf8-input-client");

    try {
      gateway.handleConnection(client);
      await waitForOutput(client, (data) => data.includes("\x1b]133;A"));

      gateway.writeInput(client, { data: "你好" });
      await delay(50);
      gateway.writeInput(client, { data: "\x7f\x7f" });
      await delay(50);

      const output = joinedOutput(client);
      assert.ok(output.includes("你好"));
      assert.ok(!output.includes("Display all"));
    } finally {
      gateway.handleDisconnect(client);
    }
  },
);

test(
  "a real shell keeps multibyte output intact across read boundaries",
  { skip: process.platform === "win32", timeout: 15_000 },
  async () => {
    const gateway = new TerminalGateway({ cwd: process.cwd() });
    const client = createClient("utf8-client");
    const expected = "中".repeat(4_000);

    try {
      gateway.handleConnection(client);
      await waitForOutput(client, (data) => data.includes("\x1b]133;A"));

      gateway.writeInput(client, {
        data: `printf '%s' "$(python3 -c 'print("中"*4000, end="")' 2>/dev/null || printf '${expected}')"\r`,
      });
      await waitForOutput(client, () => joinedOutput(client).includes(expected));

      // A chunk boundary in the middle of a 3-byte character would show up here.
      assert.ok(!joinedOutput(client).includes("�"));
    } finally {
      gateway.handleDisconnect(client);
    }
  },
);

function createGatewayHarness(overrides = {}) {
  const terminals = [];
  const harness = { cleanups: 0 };

  harness.gateway = new TerminalGateway({
    cwd: "/workspace",
    platform: "darwin",
    prepareShell: () => ({
      args: ["-i"],
      cleanup: () => {
        harness.cleanups += 1;
      },
      command: "/bin/zsh",
      env: {},
      kind: "zsh",
      supportsBlocks: true,
    }),
    spawn: () => {
      const terminal = new FakeTerminal();
      terminals.push(terminal);
      return terminal;
    },
    subshellBootstrap: () => "eval bootstrap",
    ...overrides,
  });
  harness.terminals = terminals;

  return harness;
}

function createClient(id) {
  return {
    events: [],
    id,
    emit(event, payload) {
      this.events.push([event, payload]);
    },
  };
}

function outputs(client) {
  return client.events
    .filter(([event]) => event === "terminal:output")
    .map(([, payload]) => payload.data);
}

function joinedOutput(client) {
  return outputs(client).join("");
}

async function waitForOutput(client, predicate) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const match = outputs(client).find(predicate);
    if (match !== undefined) return match;
    await delay(10);
  }

  assert.fail("timed out waiting for terminal output");
}

/** For markers that may straddle two PTY reads. */
async function waitForJoined(client, predicate) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const output = joinedOutput(client);
    if (predicate(output)) return output;
    await delay(10);
  }

  assert.fail("timed out waiting for terminal output");
}

async function waitForEvent(client, predicate) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const event = client.events.find(predicate);
    if (event) return event;
    await delay(10);
  }

  assert.fail("timed out waiting for terminal event");
}

class FakeTerminal {
  killCalls = 0;
  resizeCalls = [];
  writes = [];
  #dataHandlers = [];
  #exitHandlers = [];

  emitData(data) {
    for (const handler of this.#dataHandlers) handler(data);
  }

  emitExit(exitCode) {
    for (const handler of this.#exitHandlers) handler({ exitCode });
  }

  kill() {
    this.killCalls += 1;
  }

  onData(handler) {
    this.#dataHandlers.push(handler);
    return { dispose() {} };
  }

  onExit(handler) {
    this.#exitHandlers.push(handler);
    return { dispose() {} };
  }

  resize(cols, rows) {
    this.resizeCalls.push([cols, rows]);
  }

  write(data) {
    this.writes.push(data);
  }
}
