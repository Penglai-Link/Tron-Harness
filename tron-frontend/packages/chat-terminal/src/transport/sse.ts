import { fetchEventSource } from "@microsoft/fetch-event-source";

import { parseTerminalMessage } from "./protocol";
import type {
  AIRequestOptions,
  AIStreamOptions,
  AIStreamResult,
  AITransport,
} from "./types";

type MaybePromise<T> = T | Promise<T>;

export interface SseAITransportOptions {
  buildBody?: (content: string, stream: boolean) => unknown;
  fetch?: typeof globalThis.fetch;
  headers?: HeadersInit | (() => MaybePromise<HeadersInit>);
  openWhenHidden?: boolean;
  url: string;
}

export class SseAITransport implements AITransport {
  private readonly activeRequests = new Set<AbortController>();
  private readonly fetchImplementation: typeof globalThis.fetch;
  private disposed = false;

  constructor(private readonly options: SseAITransportOptions) {
    this.fetchImplementation =
      options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  async stream(
    content: string,
    options: AIStreamOptions,
  ): Promise<AIStreamResult> {
    const request = this.createRequest(options.signal);

    try {
      const headers = await this.createHeaders(true);
      let requestId = "";

      return await new Promise<AIStreamResult>((resolve, reject) => {
        let settled = false;
        const resolveOnce = () => {
          if (settled) return;
          settled = true;
          resolve({ requestId: requestId || "default" });
        };
        const rejectOnce = (error: unknown) => {
          if (settled) return;
          settled = true;
          reject(toError(error, "AI 流式请求失败"));
        };

        void fetchEventSource(this.options.url, {
          body: JSON.stringify(this.buildBody(content, true)),
          fetch: this.fetchImplementation,
          headers,
          method: "POST",
          onclose: resolveOnce,
          onerror: (error) => {
            rejectOnce(getRequestError(request.controller.signal, error));
            throw error;
          },
          onmessage: ({ data }) => {
            if (!data || data === "[DONE]") return;

            let payload: unknown;
            try {
              payload = JSON.parse(data);
            } catch (error) {
              rejectOnce(error);
              return;
            }

            const message = parseTerminalMessage(payload);
            if (!message) {
              rejectOnce(new Error("AI 流式响应格式错误"));
              return;
            }

            requestId = message.messageId;
            if (message.type === "TEXT_MESSAGE_CONTENT") {
              options.onChunk({ delta: message.delta, requestId });
            } else if (message.type === "TEXT_MESSAGE_END") {
              resolveOnce();
            } else if (message.type === "TEXT_MESSAGE_ERROR") {
              rejectOnce(new Error(message.delta || "AI 流式请求失败"));
            }
          },
          openWhenHidden: this.options.openWhenHidden ?? true,
          signal: request.controller.signal,
        }).catch((error) => {
          rejectOnce(getRequestError(request.controller.signal, error));
        });
      });
    } finally {
      request.dispose();
    }
  }

  async request(content: string, options: AIRequestOptions = {}) {
    const request = this.createRequest(options.signal);

    try {
      const response = await this.fetchImplementation(this.options.url, {
        body: JSON.stringify(this.buildBody(content, false)),
        headers: await this.createHeaders(false),
        method: "POST",
        signal: request.controller.signal,
      });
      const payload: unknown = await response.json();
      const message = parseTerminalMessage(payload);

      if (message?.type === "TEXT_MESSAGE_ERROR") {
        throw new Error(message.delta || "AI 请求失败");
      }
      if (!response.ok || !message) {
        throw new Error(`AI 请求失败（${response.status}）`);
      }

      return message.delta;
    } finally {
      request.dispose();
    }
  }

  dispose() {
    this.disposed = true;
    for (const controller of this.activeRequests) {
      controller.abort(new DOMException("AI transport 已销毁", "AbortError"));
    }
    this.activeRequests.clear();
  }

  private buildBody(content: string, stream: boolean) {
    return (
      this.options.buildBody?.(content, stream) ?? {
        messages: [{ content, role: "user" }],
        stream,
      }
    );
  }

  private async createHeaders(stream: boolean) {
    const configuredHeaders =
      typeof this.options.headers === "function"
        ? await this.options.headers()
        : this.options.headers;
    const headers = new Headers(configuredHeaders);

    if (!headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }
    if (!headers.has("Accept")) {
      headers.set("Accept", stream ? "text/event-stream" : "application/json");
    }

    return headers;
  }

  private createRequest(signal?: AbortSignal) {
    if (this.disposed) {
      throw new Error("AI transport 已销毁");
    }

    const controller = new AbortController();
    const handleAbort = () => {
      controller.abort(signal?.reason);
    };

    if (signal?.aborted) {
      handleAbort();
    } else {
      signal?.addEventListener("abort", handleAbort, { once: true });
    }
    this.activeRequests.add(controller);

    return {
      controller,
      dispose: () => {
        signal?.removeEventListener("abort", handleAbort);
        this.activeRequests.delete(controller);
      },
    };
  }
}

export function createSseAITransport(options: SseAITransportOptions) {
  return new SseAITransport(options);
}

function toError(error: unknown, fallback: string) {
  if (error instanceof Error) {
    return error;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "name" in error &&
    typeof error.name === "string" &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    const normalizedError = new Error(error.message);
    normalizedError.name = error.name;
    return normalizedError;
  }

  return new Error(fallback);
}

function getRequestError(signal: AbortSignal, error: unknown) {
  if (!signal.aborted) {
    return error;
  }

  return signal.reason instanceof Error
    ? signal.reason
    : new DOMException("请求已取消", "AbortError");
}
