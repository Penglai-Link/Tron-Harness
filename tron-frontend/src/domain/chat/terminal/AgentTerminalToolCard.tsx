import { createContext, useContext, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  openAgentTerminal,
  refreshAgentTerminals,
  stopAgentTerminal,
  useAgentTerminals,
} from "./store";

export const AgentTerminalSessionContext = createContext("");
function object(value: unknown): Record<string, unknown> {
  if (typeof value === "string") {
    try {
      return object(JSON.parse(value));
    } catch {
      return {};
    }
  }
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
export function AgentTerminalToolCard({
  card,
}: {
  card: { toolName: string; args: unknown; result: string; ended: boolean };
}) {
  const sessionCode = useContext(AgentTerminalSessionContext);
  const { t } = useTranslation("pages");
  const result = object(card.result);
  const args = object(card.args);
  const id =
    typeof result.terminalId === "string"
      ? result.terminalId
      : typeof args.terminalId === "string"
        ? args.terminalId
        : "";
  const session = useAgentTerminals((state) => state.sessions[sessionCode]);
  const info = session?.terminals.find((item) => item.terminalId === id);
  const [error, setError] = useState("");
  const [stopping, setStopping] = useState(false);
  useEffect(() => {
    if (!sessionCode || !card.ended) return;
    const controller = new AbortController();
    void refreshAgentTerminals(sessionCode, controller.signal).catch(() => {});
    return () => controller.abort();
  }, [sessionCode, card.ended, card.result]);
  const command =
    info?.command ??
    (typeof result.command === "string"
      ? result.command
      : typeof args.command === "string"
        ? args.command
        : "");
  const cwd =
    info?.cwd ??
    (typeof result.cwd === "string"
      ? result.cwd
      : typeof args.cwd === "string"
        ? args.cwd
        : "");
  const failed =
    Boolean(result.error) ||
    result.success === false ||
    (card.ended &&
      Boolean(card.result) &&
      !Object.keys(result).length &&
      card.toolName !== "terminal_list");
  return (
    <article className="my-2 rounded-lg border bg-muted/20 p-3 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <strong>{t(`agentTerminal.${card.toolName}`)}</strong>
        <span className="text-muted-foreground">
          {t(
            failed
              ? "agentTerminal.actionFailed"
              : card.ended
                ? "agentTerminal.actionDone"
                : "agentTerminal.actionRunning",
          )}
        </span>
        {info ? (
          <span>
            {t(info.running ? "agentTerminal.running" : "agentTerminal.exited")}
            {info.exitCode != null ? ` (${info.exitCode})` : ""}
          </span>
        ) : null}
      </div>
      {command ? (
        <p className="mt-2 break-all font-mono">
          {command}
          {info?.args.length ? ` ${info.args.join(" ")}` : ""}
        </p>
      ) : null}
      {cwd ? (
        <p className="mt-1 break-all text-xs text-muted-foreground">{cwd}</p>
      ) : null}
      {id && sessionCode ? (
        <div className="mt-2 flex gap-3">
          <button
            className="underline"
            onClick={() => openAgentTerminal(sessionCode, id)}
          >
            {t("agentTerminal.view")}
          </button>
          <button
            disabled={!info?.running || stopping}
            onClick={async () => {
              setStopping(true);
              setError("");
              try {
                await stopAgentTerminal(sessionCode, id);
              } catch {
                setError(t("agentTerminal.stopFailed"));
              } finally {
                setStopping(false);
              }
            }}
          >
            {t("agentTerminal.stop")}
          </button>
        </div>
      ) : null}
      {error ? (
        <p role="alert" className="mt-2 text-destructive-foreground">
          {error}
        </p>
      ) : null}
    </article>
  );
}
