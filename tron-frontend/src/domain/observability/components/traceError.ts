import type { TFunction } from "i18next";
import type { TraceSpan } from "../model/Trace";

function reportMessages(value: unknown): string[] {
  if (!value || typeof value !== "object") return [];
  const report = value as Record<string, unknown>;
  const messages = typeof report.message === "string" && report.message.trim()
    ? [report.message.trim()]
    : [];
  for (const key of ["causes", "cleanup"]) {
    if (Array.isArray(report[key])) messages.push(...report[key].flatMap(reportMessages));
  }
  return messages;
}

export function eventErrorMessage(event: TraceSpan["events"][number]): string | null {
  const report = event.attributes["agent.error.report"];
  if (report) {
    try {
      const messages = [...new Set(reportMessages(JSON.parse(report)))];
      if (messages.length) return messages.join("\n");
    } catch {
      // Other fields may still be usable when external reports are malformed.
    }
  }
  const message = event.attributes["exception.message"] || event.attributes["error.message"];
  return message?.trim() || null;
}

export function spanErrorMessage(span: TraceSpan, t: TFunction<"observability">): string | null {
  for (const event of span.events) {
    const message = eventErrorMessage(event);
    if (message) return message;
  }
  const browser = span.attributes["service.name"] === "tron-frontend";
  const type = span.attributes["error.type"];
  const status = Number(span.attributes["http.response.status_code"]);
  if (browser) {
    if (type === "AbortError") return t("browserFailureCancelled");
    if (type === "TimeoutError") return t("browserFailureTimeout");
    if (type === "NetworkError") return t("browserFailureNetwork");
    if (type === "ResponseUnavailable") return t("browserFailureNoResponse");
    if (type === "HttpError" && status >= 400) return t("httpFailure", { status });
  }
  for (const key of ["tron.run.error.message", "exception.message", "error.message"]) {
    const message = span.attributes[key];
    if (message?.trim()) return message.trim();
  }
  const description = span.attributes["otel.status.description"]?.trim();
  if (description && description !== type && description !== "PersonAI execution failed" && description !== "Agent run failed") {
    return description;
  }
  if (span.status === "Error") {
    if (status >= 400) return t("httpFailure", { status });
    if (browser && span.attributes["http.response.status_code"] === "0") return t("browserFailureNoResponse");
  }
  return null;
}
