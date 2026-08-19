import assert from "node:assert/strict";
import test from "node:test";

import { ChatController } from "./sse.js";

test("chat completions use the server-managed upstream authorization", async () => {
  let upstreamOptions;
  const controller = new ChatController({
    fetch: async (_url, options) => {
      upstreamOptions = options;
      return {
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "ls" } }],
        }),
      };
    },
  });
  const response = createResponse();

  await controller.chatCompletions(
    createRequest({
      messages: [{ role: "user", content: "list files" }],
      stream: false,
    }),
    response,
  );

  assert.equal(
    upstreamOptions.headers.Authorization,
    "Bearer sk-42f0b5eb8a0043509c6c48270314c90d",
  );
  assert.notEqual(upstreamOptions.headers.Authorization, "Bearer client-token");
});

test("non-streaming completions return a content message without an envelope", async () => {
  const controller = new ChatController({
    fetch: async () => ({
      ok: true,
      json: async () => ({
        choices: [{ message: { content: "ls" } }],
      }),
    }),
    now: () => 1,
  });
  const response = createResponse();

  await controller.chatCompletions(
    createRequest({ messageId: "message-1", messages: [], stream: false }),
    response,
  );

  assert.deepEqual(JSON.parse(response.body), {
    delta: "ls",
    messageId: "message-1",
    timestamp: 1,
    type: "TEXT_MESSAGE_SUGGESTION",
  });
});

test("streaming completions return content messages in each SSE frame", async () => {
  const encoder = new TextEncoder();
  const frames = [
    `data: ${JSON.stringify({ choices: [{ delta: { content: "ls" } }] })}\n\n`,
    "data: [DONE]\n\n",
  ];
  const controller = new ChatController({
    fetch: async () => ({
      body: {
        getReader() {
          let index = 0;
          return {
            async read() {
              if (index >= frames.length) return { done: true };
              return { done: false, value: encoder.encode(frames[index++]) };
            },
            async cancel() {},
            releaseLock() {},
          };
        },
      },
      ok: true,
    }),
    now: () => 1,
  });
  const response = createResponse();

  await controller.chatCompletions(
    createRequest({ messageId: "message-1", messages: [], stream: true }),
    response,
  );

  assert.deepEqual(
    response.writes.map((frame) => JSON.parse(frame.slice(6))),
    [
      {
        delta: "",
        messageId: "message-1",
        timestamp: 1,
        type: "TEXT_MESSAGE_START",
      },
      {
        delta: "ls",
        messageId: "message-1",
        timestamp: 1,
        type: "TEXT_MESSAGE_CONTENT",
      },
      {
        delta: "",
        messageId: "message-1",
        timestamp: 1,
        type: "TEXT_MESSAGE_END",
      },
    ],
  );
});

function createRequest(body) {
  return {
    headers: {
      authorization: "Bearer client-token",
      "content-type": "application/json",
    },
    async *[Symbol.asyncIterator]() {
      yield Buffer.from(JSON.stringify(body));
    },
  };
}

function createResponse() {
  return {
    headers: {},
    writes: [],
    once() {},
    setHeader(name, value) {
      this.headers[name] = value;
    },
    end(body) {
      this.body = body;
      this.writableEnded = true;
    },
    write(body) {
      this.writes.push(body);
    },
  };
}
