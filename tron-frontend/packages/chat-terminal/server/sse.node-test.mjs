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
    once() {},
    setHeader(name, value) {
      this.headers[name] = value;
    },
    end(body) {
      this.body = body;
      this.writableEnded = true;
    },
    write() {},
  };
}
