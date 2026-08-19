import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";

import { MessageType } from "./message.js";
import { TerminalGateway } from "./scoket.js";

const RECORD_SEPARATOR = "\x1e";
const UNIT_SEPARATOR = "\x1f";

test("one socket reuses one PTY and reports the resulting directory", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  assert.equal(harness.terminals.length, 1);

  const terminal = harness.terminals[0];
  terminal.emitData(sessionReady("/workspace"));
  assert.deepEqual(client.events[0], [
    "terminal:session",
    message("TEXT_MESSAGE_CONTENT", "session-1", "", "/workspace"),
  ]);

  harness.gateway.executeCommand(client, {
    command: "cd packages",
    messageId: "message-1",
  });
  const firstToken = readCommandToken(terminal.writes.at(-1));
  terminal.emitData(commandEnd(firstToken, 0, "/workspace/packages"));

  harness.gateway.executeCommand(client, {
    command: "pwd",
    messageId: "message-2",
  });
  const secondToken = readCommandToken(terminal.writes.at(-1));
  terminal.emitData(`/workspace/packages\r\n${commandEnd(
    secondToken,
    0,
    "/workspace/packages",
  )}`);

  assert.equal(harness.terminals.length, 1);
  assert.deepEqual(commandMessages(client), [
    message(MessageType.START, "message-1"),
    message(MessageType.END, "message-1", "", "/workspace/packages"),
    message(MessageType.START, "message-2"),
    message(
      MessageType.CONTENT,
      "message-2",
      "/workspace/packages\r\n",
    ),
    message(MessageType.END, "message-2", "", "/workspace/packages"),
  ]);
});

test("session startup failures use the content error message format", () => {
  const gateway = new TerminalGateway({
    createMessageId: () => "session-error-1",
    now: () => 1,
    spawn: () => {
      throw new Error("终端会话无法启动");
    },
  });
  const client = createClient("client-a");

  gateway.handleConnection(client);

  assert.deepEqual(client.events, [
    [
      "terminal:session",
      {
        delta: "终端会话无法启动",
        messageId: "session-error-1",
        timestamp: 1,
        type: "TEXT_MESSAGE_ERROR",
      },
    ],
  ]);
});

test("socket sessions are isolated and disconnect cleans only its own PTY", () => {
  const harness = createGatewayHarness();
  const firstClient = createClient("client-a");
  const secondClient = createClient("client-b");

  harness.gateway.handleConnection(firstClient);
  harness.gateway.handleConnection(secondClient);
  assert.equal(harness.terminals.length, 2);

  const [firstTerminal, secondTerminal] = harness.terminals;
  firstTerminal.emitData(sessionReady("/workspace/a"));
  secondTerminal.emitData(sessionReady("/workspace/b"));

  harness.gateway.handleDisconnect(firstClient);

  assert.equal(firstTerminal.killCalls, 1);
  assert.equal(secondTerminal.killCalls, 0);
  assert.equal(harness.gateway.sessionsByClient.has("client-a"), false);
  assert.equal(harness.gateway.sessionsByClient.has("client-b"), true);
});

test("a session rejects concurrent commands and Ctrl+C keeps the PTY alive", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];
  terminal.emitData(sessionReady("/workspace"));

  harness.gateway.executeCommand(client, {
    command: "sleep 10",
    messageId: "message-1",
  });
  harness.gateway.executeCommand(client, {
    command: "pwd",
    messageId: "message-2",
  });
  harness.gateway.cancelCommand(client, { messageId: "message-1" });

  assert.equal(terminal.writes.at(-1), "\x03");
  assert.equal(terminal.killCalls, 0);
  assert.deepEqual(commandMessages(client).at(-1),
    message(MessageType.ERROR, "message-2", "已有命令正在执行"),
  );
});

test("interactive input and resize target only the active PTY command", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];
  terminal.emitData(sessionReady("/workspace"));
  harness.gateway.executeCommand(client, {
    command: "vim edit.txt",
    interactive: true,
    messageId: "message-1",
  });

  harness.gateway.writeCommandInput(client, {
    data: "i",
    messageId: "message-1",
  });
  harness.gateway.writeCommandInput(client, {
    data: "ignored",
    messageId: "message-2",
  });
  harness.gateway.resizeTerminal(client, { cols: 120, rows: 50 });

  assert.equal(terminal.writes.at(-1), "i");
  assert.deepEqual(terminal.resizeCalls, [[120, 50]]);
});

test("a marker split across PTY chunks never leaks into command output", () => {
  const harness = createGatewayHarness();
  const client = createClient("client-a");

  harness.gateway.handleConnection(client);
  const terminal = harness.terminals[0];
  terminal.emitData(sessionReady("/workspace"));
  harness.gateway.executeCommand(client, {
    command: "printf result",
    messageId: "message-1",
  });

  const token = readCommandToken(terminal.writes.at(-1));
  terminal.emitData(`result${RECORD_SEPARATOR}TRON_COMMAND_`);
  terminal.emitData(`END:${token}:0:/workspace${UNIT_SEPARATOR}\r\n`);

  assert.deepEqual(commandMessages(client), [
    message(MessageType.START, "message-1"),
    message(MessageType.CONTENT, "message-1", "result"),
    message(MessageType.END, "message-1", "", "/workspace"),
  ]);
});

test(
  "a real PTY echoes prompt input while a command runs",
  {
    skip: process.platform === "win32",
    timeout: 10_000,
  },
  async () => {
    const gateway = new TerminalGateway({ cwd: process.cwd() });
    const client = createClient("prompt-input-client");

    try {
      gateway.handleConnection(client);
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "terminal:session" &&
          payload.type === "TEXT_MESSAGE_CONTENT" &&
          payload.delta === "",
      );

      gateway.executeCommand(client, {
        command:
          "printf 'package name: '; read package_name; printf 'created:%s' \"$package_name\"",
        interactive: true,
        messageId: "prompt-input-message",
      });
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "prompt-input-message" &&
          payload.type === MessageType.CONTENT &&
          payload.delta.includes("package name: "),
      );

      gateway.writeCommandInput(client, {
        data: "vue-demo\r",
        messageId: "prompt-input-message",
      });
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "prompt-input-message" &&
          payload.type === MessageType.END,
      );

      const output = commandMessages(client)
        .filter(
          (payload) =>
            payload.messageId === "prompt-input-message" &&
            payload.type === MessageType.CONTENT,
        )
        .map((payload) => payload.delta)
        .join("");
      assert.equal(output, "package name: vue-demo\r\ncreated:vue-demo");
    } finally {
      gateway.handleDisconnect(client);
    }
  },
);

test(
  "a real PTY preserves its directory across commands",
  { skip: process.platform === "win32" },
  async () => {
    const initialDirectory = process.cwd();
    const expectedDirectory = path.join(initialDirectory, "node_modules");
    const gateway = new TerminalGateway({ cwd: initialDirectory });
    const client = createClient("real-client");

    try {
      gateway.handleConnection(client);
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "terminal:session" &&
          payload.type === "TEXT_MESSAGE_CONTENT" &&
          payload.delta === "",
      );

      gateway.executeCommand(client, {
        command: "cd node_modules",
        messageId: "real-message-1",
      });
      const firstEnd = await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "real-message-1" &&
          payload.type === MessageType.END,
      );
      assert.equal(firstEnd[1].currentDirectory, expectedDirectory);

      gateway.executeCommand(client, {
        command: "pwd",
        messageId: "real-message-2",
      });
      const secondEnd = await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "real-message-2" &&
          payload.type === MessageType.END,
      );
      assert.equal(secondEnd[1].currentDirectory, expectedDirectory);

      const output = commandMessages(client)
        .filter(
          (payload) =>
            payload.messageId === "real-message-2" &&
            payload.type === MessageType.CONTENT,
        )
        .map((payload) => payload.delta)
        .join("");
      assert.equal(output, `${expectedDirectory}\r\n`);
    } finally {
      gateway.handleDisconnect(client);
    }

    assert.equal(gateway.sessionsByClient.has(client.id), false);
  },
);

test(
  "a real zsh session loads user aliases",
  {
    skip:
      process.platform === "win32" ||
      spawnSync("zsh", ["--version"], { stdio: "ignore" }).status !== 0,
    timeout: 10_000,
  },
  async () => {
    const initialDirectory = await fs.mkdtemp(
      path.join(os.tmpdir(), "tron-zsh-test-"),
    );
    const homeDirectory = path.join(initialDirectory, "home");
    await fs.mkdir(homeDirectory);
    await fs.writeFile(
      path.join(homeDirectory, ".zshrc"),
      "alias tron_test_alias='printf alias-loaded'\n",
    );
    const gateway = new TerminalGateway({
      cwd: initialDirectory,
      env: { ...process.env, HOME: homeDirectory, SHELL: "/bin/zsh" },
      shell: "/bin/zsh",
    });
    const client = createClient("zsh-alias-client");

    try {
      gateway.handleConnection(client);
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "terminal:session" &&
          payload.type === "TEXT_MESSAGE_CONTENT" &&
          payload.delta === "",
      );

      gateway.executeCommand(client, {
        command: "tron_test_alias",
        messageId: "zsh-alias-message",
      });
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "zsh-alias-message" &&
          payload.type === MessageType.END,
      );

      const output = commandMessages(client)
        .filter(
          (payload) =>
            payload.messageId === "zsh-alias-message" &&
            payload.type === MessageType.CONTENT,
        )
        .map((payload) => payload.delta)
        .join("");
      assert.equal(output, "alias-loaded");
    } finally {
      gateway.handleDisconnect(client);
      await fs.rm(initialDirectory, { force: true, recursive: true });
    }
  },
);

test(
  "a real PTY accepts Vim input without consuming the command marker",
  {
    skip:
      process.platform === "win32" ||
      spawnSync("vim", ["--version"], { stdio: "ignore" }).status !== 0,
    timeout: 10_000,
  },
  async () => {
    const initialDirectory = await fs.mkdtemp(
      path.join(os.tmpdir(), "tron-vim-test-"),
    );
    const gateway = new TerminalGateway({ cwd: initialDirectory });
    const client = createClient("vim-client");

    try {
      gateway.handleConnection(client);
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "terminal:session" &&
          payload.type === "TEXT_MESSAGE_CONTENT" &&
          payload.delta === "",
      );

      gateway.executeCommand(client, {
        command: "vim -Nu NONE -n edit.txt",
        interactive: true,
        messageId: "vim-message",
      });
      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "vim-message" &&
          payload.type === MessageType.CONTENT,
      );

      gateway.writeCommandInput(client, {
        data: "ihello from tron",
        messageId: "vim-message",
      });
      gateway.writeCommandInput(client, {
        data: "\x1b:wq\r",
        messageId: "vim-message",
      });

      await waitForEvent(
        client,
        ([event, payload]) =>
          event === "command:message" &&
          payload.messageId === "vim-message" &&
          payload.type === MessageType.END,
      );
      assert.equal(
        await fs.readFile(path.join(initialDirectory, "edit.txt"), "utf8"),
        "hello from tron\n",
      );
    } finally {
      gateway.handleDisconnect(client);
      await fs.rm(initialDirectory, { force: true, recursive: true });
    }
  },
);

function createGatewayHarness() {
  const terminals = [];
  const gateway = new TerminalGateway({
    createMessageId: () => "session-1",
    now: () => 1,
    spawn: () => {
      const terminal = new FakeTerminal();
      terminals.push(terminal);
      return terminal;
    },
  });

  return { gateway, terminals };
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

function commandMessages(client) {
  return client.events
    .filter(([event]) => event === "command:message")
    .map(([, payload]) => payload);
}

function message(type, messageId, delta = "", currentDirectory) {
  return {
    delta,
    messageId,
    timestamp: 1,
    type,
    ...(currentDirectory ? { currentDirectory } : {}),
  };
}

function sessionReady(currentDirectory) {
  return `${RECORD_SEPARATOR}TRON_SESSION_READY:${currentDirectory}${UNIT_SEPARATOR}\r\n`;
}

function commandEnd(token, exitCode, currentDirectory) {
  return `${RECORD_SEPARATOR}TRON_COMMAND_END:${token}:${exitCode}:${currentDirectory}${UNIT_SEPARATOR}\r\n`;
}

function readCommandToken(command) {
  const token = command?.match(/TRON_COMMAND_END:([^:]+):/)?.[1];
  assert.ok(token, "command frame contains an end marker token");
  return token;
}

async function waitForEvent(client, predicate) {
  const deadline = Date.now() + 3_000;
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

  emitExit(exitCode = 0) {
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
