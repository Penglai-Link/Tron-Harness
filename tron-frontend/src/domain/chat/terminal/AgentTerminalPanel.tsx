import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { requestApi } from "@/shared/api/httpClient";
import {
  useAgentTerminals,
  terminalEndpoint,
  appendTerminalOutput,
  closeAgentTerminal,
  openAgentTerminal,
  refreshAgentTerminals,
  stopAgentTerminal,
  type TerminalOutput,
} from "./store";
import "@xterm/xterm/css/xterm.css";
function Output({ text }: { text: string }) {
  const { t } = useTranslation("pages");
  const host = useRef<HTMLDivElement>(null);
  const term = useRef<Terminal | null>(null);
  const previous = useRef("");
  useEffect(() => {
    if (!host.current) return;
    const instance = new Terminal({
      disableStdin: true,
      convertEol: true,
      scrollback: 3000,
      fontSize: 12,
      theme: { background: "#111111", foreground: "#eeeeee" },
    });
    const fit = new FitAddon();
    instance.loadAddon(fit);
    instance.open(host.current);
    instance.textarea?.setAttribute("aria-label", t("agentTerminal.output"));
    instance.textarea?.setAttribute("aria-readonly", "true");
    const resize = new ResizeObserver(() => {
      try {
        fit.fit();
      } catch {
        /* hidden panel */
      }
    });
    resize.observe(host.current);
    term.current = instance;
    previous.current = "";
    return () => {
      resize.disconnect();
      instance.dispose();
      term.current = null;
    };
  }, [t]);
  useEffect(() => {
    if (!term.current) return;
    if (text.startsWith(previous.current)) {
      term.current.write(text.slice(previous.current.length));
    } else {
      term.current.reset();
      term.current.write(text);
    }
    previous.current = text;
  }, [text, t]);
  return (
    <div
      ref={host}
      className="h-full min-h-0 flex-1 overflow-hidden bg-[#111] p-2"
    />
  );
}
export function AgentTerminalPanel({
  sessionCode,
  visible,
  onShowManual,
}: {
  sessionCode: string;
  visible: boolean;
  onShowManual?: () => void;
}) {
  const { t } = useTranslation("pages");
  const session = useAgentTerminals((state) => state.sessions[sessionCode]);
  const [error, setError] = useState("");
  const [stopping, setStopping] = useState(false);
  const selected = session?.selected ?? "";
  const info = session?.terminals.find((item) => item.terminalId === selected);
  const log = session?.logs[selected];
  useEffect(() => {
    if (!sessionCode) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        await refreshAgentTerminals(sessionCode, controller.signal);
      } catch {
        if (!controller.signal.aborted)
          useAgentTerminals.getState().update(sessionCode, (value) => ({
            ...value,
            error: t("agentTerminal.unavailable"),
          }));
      }
      if (!controller.signal.aborted) timer = setTimeout(refresh, 2000);
    };
    void refresh();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [sessionCode, t]);
  useEffect(() => {
    setError("");
    if (!sessionCode || !selected || !visible) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const read = async () => {
      try {
        const cursor =
          useAgentTerminals.getState().sessions[sessionCode]?.logs[selected]
            ?.cursor ?? 0;
        const result = await requestApi<TerminalOutput>(
          terminalEndpoint(sessionCode, selected),
          { queryParams: { cursor }, signal: controller.signal },
        );
        if (controller.signal.aborted) return;
        appendTerminalOutput(sessionCode, selected, result);
        if (result.running || !result.outputClosed || result.output.length)
          timer = setTimeout(read, result.output.length ? 100 : 1000);
      } catch {
        if (!controller.signal.aborted)
          setError(t("agentTerminal.unavailable"));
      }
    };
    void read();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [sessionCode, selected, visible, t]);
  return (
    <section
      className="flex h-full min-h-0 flex-col"
      aria-label={t("agentTerminal.title")}
    >
      <div
        className="flex shrink-0 items-center gap-2 overflow-x-auto border-b px-2 py-1"
        role="tablist"
        aria-label={t("agentTerminal.title")}
      >
        {onShowManual ? (
          <button className="shrink-0 px-2 text-sm" onClick={onShowManual}>
            {t("agentTerminal.manual")}
          </button>
        ) : null}
        {(session?.tabs ?? []).map((id) => (
          <div key={id} className="flex shrink-0 items-center rounded border">
            <button
              role="tab"
              aria-selected={selected === id}
              className="max-w-64 truncate px-3 py-1 text-sm"
              onClick={() =>
                useAgentTerminals
                  .getState()
                  .update(sessionCode, (value) => ({ ...value, selected: id }))
              }
            >
              {session?.terminals.find((item) => item.terminalId === id)
                ?.command ?? t("agentTerminal.title")}
            </button>
            <button
              className="px-2"
              title={t("agentTerminal.closeHint")}
              aria-label={t("agentTerminal.close")}
              onClick={() => closeAgentTerminal(sessionCode, id)}
            >
              ×
            </button>
          </div>
        ))}
        <select
          aria-label={t("agentTerminal.open")}
          value=""
          className="max-w-64 rounded border p-1 text-sm"
          onChange={(event) => {
            if (event.target.value)
              openAgentTerminal(sessionCode, event.target.value);
          }}
        >
          <option value="">{t("agentTerminal.open")}</option>
          {(session?.terminals ?? []).map((item) => (
            <option value={item.terminalId} key={item.terminalId}>
              {item.command} · {item.cwd} ·{" "}
              {t(
                item.running ? "agentTerminal.running" : "agentTerminal.exited",
              )}
            </option>
          ))}
        </select>
      </div>
      {selected ? (
        <>
          <div className="flex shrink-0 items-center gap-3 px-3 py-1 text-xs">
            <span>
              {t(
                info?.running
                  ? "agentTerminal.running"
                  : info
                    ? "agentTerminal.exited"
                    : "agentTerminal.unavailable",
              )}
              {info?.exitCode != null ? ` (${info.exitCode})` : ""}
            </span>
            <span className="min-w-0 flex-1 truncate" title={info?.cwd}>
              {info?.cwd}
            </span>
            <span>{t("agentTerminal.readOnly")}</span>
            <button
              disabled={!info?.running || stopping}
              onClick={async () => {
                setStopping(true);
                try {
                  await stopAgentTerminal(sessionCode, selected);
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
          {error || session?.error ? (
            <p role="alert" className="px-3 text-sm text-red-600">
              {error || session?.error}
            </p>
          ) : null}
          {log?.truncated ? (
            <p className="px-3 text-xs">{t("agentTerminal.truncated")}</p>
          ) : null}
          <Output key={`${sessionCode}:${selected}`} text={log?.text ?? ""} />
        </>
      ) : (
        <p className="p-4 text-sm text-muted-foreground">
          {t("agentTerminal.empty")}
        </p>
      )}
    </section>
  );
}
