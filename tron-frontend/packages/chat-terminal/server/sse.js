import "reflect-metadata";

import { Controller, Post, Req, Res } from "@nestjs/common";

import { MessageType, createMessage, resolveMessageId } from "./message.js";

export const CHAT_COMPLETIONS_PATH = "/api/llm/v1/chat/completions";
export const UPSTREAM_CHAT_COMPLETIONS_URL =
  "https://qa-gewu.cisdigital.cn/api/llm/v1/chat/completions";

const MAX_REQUEST_BODY_BYTES = 1024 * 1024;

export class ChatController {
  constructor(options = {}) {
    this.fetch = options.fetch ?? globalThis.fetch;
    this.now = options.now ?? Date.now;
  }

  async chatCompletions(request, response) {
    let body;

    try {
      body = await readJsonBody(request);
    } catch (error) {
      writeJsonMessage(
        response,
        createMessage({
          delta: getErrorMessage(error, "请求体格式错误"),
          messageId: request.headers["x-message-id"],
          now: this.now,
          type: MessageType.ERROR,
        }),
        400,
      );
      return;
    }

    if (!isRecord(body)) {
      writeJsonMessage(
        response,
        createMessage({
          delta: "请求体必须是 JSON 对象",
          messageId: request.headers["x-message-id"],
          now: this.now,
          type: MessageType.ERROR,
        }),
        400,
      );
      return;
    }

    const messageId = resolveMessageId(
      body.messageId ?? request.headers["x-message-id"],
    );
    const stream = body.stream !== false;
    const upstreamBody = { ...body, model: "Qwen3-32B" };
    delete upstreamBody.messageId;
    upstreamBody.stream = stream;

    const abortController = new AbortController();
    response.once("close", () => {
      if (!response.writableEnded) abortController.abort();
    });

    let upstream;
    try {
      upstream = await this.fetch(UPSTREAM_CHAT_COMPLETIONS_URL, {
        body: JSON.stringify(upstreamBody),
        headers: {
          "Content-Type": request.headers["content-type"] ?? "application/json",
          ...(request.headers.authorization
            ? { Authorization: request.headers.authorization }
            : {}),
          Accept: stream ? "text/event-stream" : "application/json",
        },
        method: "POST",
        signal: abortController.signal,
      });
    } catch (error) {
      this.writeRequestError(response, stream, messageId, error, 502);
      return;
    }

    if (!upstream.ok) {
      this.writeRequestError(
        response,
        stream,
        messageId,
        new Error(`大模型服务请求失败（${upstream.status}）`),
        upstream.status || 502,
      );
      return;
    }

    if (stream) {
      await this.streamResponse(response, upstream, messageId);
      return;
    }

    await this.suggestionResponse(response, upstream, messageId);
  }

  async streamResponse(response, upstream, messageId) {
    setSseHeaders(response);
    writeSseMessage(
      response,
      createMessage({ messageId, now: this.now, type: MessageType.START }),
    );

    try {
      await readUpstreamSse(upstream, (delta) => {
        writeSseMessage(
          response,
          createMessage({
            delta,
            messageId,
            now: this.now,
            type: MessageType.CONTENT,
          }),
        );
      });
      writeSseMessage(
        response,
        createMessage({ messageId, now: this.now, type: MessageType.END }),
      );
    } catch (error) {
      writeSseMessage(
        response,
        createMessage({
          delta: getErrorMessage(error, "大模型流读取失败"),
          messageId,
          now: this.now,
          type: MessageType.ERROR,
        }),
      );
    } finally {
      response.end();
    }
  }

  async suggestionResponse(response, upstream, messageId) {
    try {
      const payload = await upstream.json();
      const suggestion = payload?.choices?.[0]?.message?.content;

      if (typeof suggestion !== "string") {
        throw new Error("大模型响应缺少建议命令");
      }

      writeJsonMessage(
        response,
        createMessage({
          delta: suggestion,
          messageId,
          now: this.now,
          type: MessageType.SUGGESTION,
        }),
      );
    } catch (error) {
      writeJsonMessage(
        response,
        createMessage({
          delta: getErrorMessage(error, "大模型响应解析失败"),
          messageId,
          now: this.now,
          type: MessageType.ERROR,
        }),
        502,
      );
    }
  }

  writeRequestError(response, stream, messageId, error, statusCode) {
    const message = createMessage({
      delta: getErrorMessage(error, "大模型服务请求失败"),
      messageId,
      now: this.now,
      type: MessageType.ERROR,
    });

    response.statusCode = statusCode;
    if (stream) {
      setSseHeaders(response, statusCode);
      writeSseMessage(response, message);
      response.end();
      return;
    }

    writeJsonMessage(response, message, statusCode);
  }
}

async function readJsonBody(request) {
  const chunks = [];
  let totalBytes = 0;

  for await (const chunk of request) {
    const buffer = Buffer.from(chunk);
    totalBytes += buffer.length;

    if (totalBytes > MAX_REQUEST_BODY_BYTES) {
      throw new Error("请求体不能超过 1 MB");
    }

    chunks.push(buffer);
  }

  const text = Buffer.concat(chunks).toString("utf8");
  if (!text.trim()) throw new Error("请求体不能为空");
  return JSON.parse(text);
}

async function readUpstreamSse(upstream, onDelta) {
  if (!upstream.body) throw new Error("大模型服务未返回响应流");

  const reader = upstream.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let completed = false;

  try {
    while (!completed) {
      const { done, value } = await reader.read();
      buffer += done
        ? decoder.decode()
        : decoder.decode(value, { stream: true });

      let boundary = findFrameBoundary(buffer);
      while (boundary) {
        const event = buffer.slice(0, boundary.index);
        buffer = buffer.slice(boundary.index + boundary.length);
        completed = consumeUpstreamEvent(event, onDelta) || completed;
        if (completed) break;
        boundary = findFrameBoundary(buffer);
      }

      if (done) {
        if (!completed && buffer.trim()) consumeUpstreamEvent(buffer, onDelta);
        break;
      }
    }
  } finally {
    if (completed) await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}

function consumeUpstreamEvent(event, onDelta) {
  const data = event
    .split(/\r?\n/)
    .filter((line) => line.startsWith("data:"))
    .map((line) => line.slice(5).trimStart())
    .join("\n");

  if (!data) return false;
  if (data === "[DONE]") return true;

  let payload;
  try {
    payload = JSON.parse(data);
  } catch {
    throw new Error("大模型服务返回了无效的 SSE 数据");
  }

  if (payload?.error) {
    throw new Error(
      typeof payload.error.message === "string"
        ? payload.error.message
        : "大模型服务返回错误",
    );
  }

  const delta = payload?.choices?.[0]?.delta?.content;
  if (typeof delta === "string" && delta) onDelta(delta);
  return false;
}

function findFrameBoundary(buffer) {
  const match = /\r?\n\r?\n/.exec(buffer);
  return match ? { index: match.index, length: match[0].length } : undefined;
}

function setSseHeaders(response, statusCode = 200) {
  response.statusCode = statusCode;
  response.setHeader("Content-Type", "text/event-stream; charset=utf-8");
  response.setHeader("Cache-Control", "no-cache, no-transform");
  response.setHeader("Connection", "keep-alive");
}

function writeSseMessage(response, message) {
  response.write(`data: ${JSON.stringify(message)}\n\n`);
}

function writeJsonMessage(response, message, statusCode = 200) {
  response.statusCode = statusCode;
  response.setHeader("Content-Type", "application/json; charset=utf-8");
  response.end(JSON.stringify(message));
}

function getErrorMessage(error, fallback) {
  return error instanceof Error && error.message ? error.message : fallback;
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

Controller()(ChatController);
Req()(ChatController.prototype, "chatCompletions", 0);
Res()(ChatController.prototype, "chatCompletions", 1);
Post(CHAT_COMPLETIONS_PATH.slice(1))(
  ChatController.prototype,
  "chatCompletions",
  Object.getOwnPropertyDescriptor(ChatController.prototype, "chatCompletions"),
);
