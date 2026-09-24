import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { requestApi } from "@/shared/api/httpClient";
import { useTheme } from "@/shared/theme/ThemeProvider";
import { getTerminalAnsiTheme } from "@/components/terminal/terminalTheme";
import {
  useAgentTerminals,
  terminalEndpoint,
  appendTerminalOutput,
  stopAgentTerminal,
  type TerminalOutput,
} from "./store";
import "@xterm/xterm/css/xterm.css";
function Output({ text }: { text: string }) {
  const { t } = useTranslation("pages");
  const { theme, activeSkin } = useTheme();
  const host = useRef<HTMLDivElement>(null);
  const term = useRef<Terminal | null>(null);
  const previous = useRef("");
  const currentTheme = useRef(theme);
  currentTheme.current = theme;
  useEffect(() => {
    if (!host.current) return;
    const instance = new Terminal({
      disableStdin: true,
      convertEol: true,
      scrollback: 3000,
      fontSize: 12,
      theme: resolveTheme(host.current, currentTheme.current),
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
    if (!host.current || !term.current) return;
    term.current.options.theme = resolveTheme(host.current, theme);
  }, [theme, activeSkin]);
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
      className="h-full min-h-0 flex-1 overflow-hidden bg-[var(--color-elevated-surface)] p-2 text-[var(--color-on-surface)]"
    />
  );
}

function resolveTheme(host: HTMLElement, theme: string) {
  const colors = getComputedStyle(host);
  const dark = theme !== "light";
  const background = colors.backgroundColor;
  const foreground = colors.color;
  const fallbackForeground = dark ? "#e4ffe8" : "#1c1b1b";
  return {
    ...getTerminalAnsiTheme(theme as "light" | "matrix-dark"),
    background:
      !background || background === "transparent" || background === "rgba(0, 0, 0, 0)"
        ? dark ? "#09150d" : "#ffffff"
        : background,
    foreground: foreground && foreground !== "canvastext" && !(dark && foreground === "rgb(0, 0, 0)")
      ? foreground
      : fallbackForeground,
    cursor: foreground && foreground !== "canvastext" && !(dark && foreground === "rgb(0, 0, 0)")
      ? foreground
      : fallbackForeground,
  };
}
export function AgentTerminalTab({
  sessionCode,
  terminalId,
  visible,
}: {
  sessionCode: string;
  terminalId: string;
  visible: boolean;
}) {
  const { t } = useTranslation("pages");
  const session = useAgentTerminals((state) => state.sessions[sessionCode]);
  const [error, setError] = useState("");
  const [stopping, setStopping] = useState(false);
  const info = session?.terminals.find((item) => item.terminalId === terminalId);
  const log = session?.logs[terminalId];
  useEffect(() => {
    setError("");
    if (!sessionCode || !terminalId || !visible) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const read = async () => {
      try {
        const cursor =
          useAgentTerminals.getState().sessions[sessionCode]?.logs[terminalId]
            ?.cursor ?? 0;
        const result = await requestApi<TerminalOutput>(
          terminalEndpoint(sessionCode, terminalId),
          { queryParams: { cursor }, signal: controller.signal },
        );
        if (controller.signal.aborted) return;
        appendTerminalOutput(sessionCode, terminalId, result);
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
  }, [sessionCode, terminalId, visible, t]);
  return (
    <section
      className="flex h-full min-h-0 flex-col"
      aria-label={t("agentTerminal.title")}
    >
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
              await stopAgentTerminal(sessionCode, terminalId);
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
        <p role="alert" className="px-3 text-sm text-destructive-foreground">
          {error || session?.error}
        </p>
      ) : null}
      {log?.truncated ? (
        <p className="px-3 text-xs">{t("agentTerminal.truncated")}</p>
      ) : null}
      <Output text={log?.text ?? ""} />
    </section>
  );
}
