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

export function spanErrorMessage(span: TraceSpan): string | null {
  for (const event of span.events) {
    const report = event.attributes["agent.error.report"];
    if (report) {
      try {
        const messages = [...new Set(reportMessages(JSON.parse(report)))];
        if (messages.length) return messages.join("\n");
      } catch {
        // External telemetry can contain malformed reports; other fields may still be usable.
      }
    }
    const message = event.attributes["exception.message"] || event.attributes["error.message"];
    if (message?.trim()) return message.trim();
  }
  for (const key of ["tron.run.error.message", "exception.message", "error.message"]) {
    const message = span.attributes[key];
    if (message?.trim()) return message.trim();
  }
  const description = span.attributes["otel.status.description"]?.trim();
  const type = span.attributes["error.type"];
  if (description && description !== type && description !== "Dude execution failed" && description !== "Agent run failed") {
    return description;
  }
  return null;
}
