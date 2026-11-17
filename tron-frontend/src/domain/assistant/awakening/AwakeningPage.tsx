import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { AgentAvatar } from "@/domain/agent/components/AgentAvatarPicker";
import { useAssistants } from "../hooks/useAssistants";
import { Button } from "@/components/ui/button";
import { AgentChatWindow } from "@/domain/chat/components/AgentChatWindow";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import type { ChatSession, ChatRuntimeRunState, SendChatMessageInput } from "@tron/chat-core";
import { getApiAccessToken } from "@/shared/api/httpClient";
import { applyCurrentLocaleHeader } from "@/i18n";
import { SoulFields } from "../components/SoulFields";
import {
  beginAwakening,
  deferAwakening,
  getAwakening,
  getAssistantSoul,
  listAssistantEvents,
  resolveMainAssistantSession,
  saveAssistantSoul,
  type AwakeningStatus,
} from "../api/assistantApi";
import { BlackHole } from "./BlackHole";
import { useAwakeningBackdrop } from "./HomeArrivalVeil";
import { awaitAwakeningRun } from "./awaitAwakeningRun";
import { useMotionPreference } from "@/shared/motion/motionPreferences";
import "./awakening.css";
type Phase = "approach" | "black" | "rain" | "dialog" | "exit";
export default function AwakeningPage() {
  const { t } = useTranslation("pages");
  const navigate = useNavigate();
  const { showRain, beginArrival } = useAwakeningBackdrop();
  const [params] = useSearchParams();
  const queryClient = useQueryClient();
  const assistants = useAssistants();
  const avatarId = assistants.data?.find((d) => d.isMain)?.avatarId ?? "neo";
  const arrival = useRef(params.get("arrival") === "1");
  const insideArrival = useRef(params.get("arrival") === "inside");
  const [phase, setPhase] = useState<Phase>(
    insideArrival.current ? "black" : arrival.current ? "approach" : "dialog"
  );
  const [playEntrance, setPlayEntrance] = useState(false);
  const [status, setStatus] = useState<AwakeningStatus | null>(null);
  const [session, setSession] = useState<ChatSession | null>(null);
  const [pendingSession, setPendingSession] = useState<ChatSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [manual, setManual] = useState(false);
  const [name, setName] = useState("");
  const [markdown, setMarkdown] = useState("");
  const [saving, setSaving] = useState(false);
  const [retry, setRetry] = useState(0);
  const [runState, setRunState] = useState<ChatRuntimeRunState | null>(null);
  const [manualComplete, setManualComplete] = useState(false);
  const leaving = useRef(false);
  const opening = useRef<Promise<
    AwakeningStatus & { playEntrance?: boolean }
  > | null>(null);
  const motion = useMotionPreference();
  const reduced =
    motion !== "system" ||
    matchMedia("(prefers-reduced-motion: reduce)").matches;
  useLayoutEffect(() => {
    showRain(phase === "rain" || phase === "dialog" || phase === "exit");
  }, [phase, showRain]);
  const adapter = useMemo(
    () =>
      createInternalChatAdapter({
        workspaceCode: null,
        getAccessToken: getApiAccessToken,
        getHeaders: () => {
          const h = applyCurrentLocaleHeader(new Headers());
          if (status) h.set("X-Assistant-Code", status.assistantCode);
          return h;
        },
      }),
    [status?.assistantCode]
  );
  useEffect(() => {
    if (!insideArrival.current) return;
    const rain = setTimeout(() => setPhase("rain"), 200);
    const dialog = setTimeout(
      () => {
        insideArrival.current = false;
        setPhase("dialog");
        navigate("/awakening", { replace: true });
      },
      reduced ? 300 : 900
    );
    return () => {
      clearTimeout(rain);
      clearTimeout(dialog);
    };
  }, [navigate, reduced]);
  useEffect(() => {
    if (!playEntrance) return;
    // The plunge always plays; only the rain/dialog transitions follow motion preferences.
    const timers = [
      setTimeout(() => setPhase("black"), 1800),
      setTimeout(() => setPhase("rain"), 2000),
      setTimeout(
        () => {
          setPhase("dialog");
          navigate("/awakening", { replace: true });
        },
        reduced ? 2100 : 2700
      ),
    ];
    return () => timers.forEach(clearTimeout);
  }, [navigate, reduced, playEntrance]);
  useEffect(() => {
    let active = true;
    setError(null);
    if (!opening.current)
      opening.current = getAwakening().then((current) =>
        current.state === "Ready" || (current.started && !current.canResume)
          ? current
          : beginAwakening()
      );
    void opening.current
      .then(async (s) => {
        if (s.started && !s.canResume && s.state !== "Ready") {
          if (active) {
            setStatus(s);
            setManual(true);
            setPhase("dialog");
          }
          return;
        }
        if (!active) return;
        setStatus(s);
        if (arrival.current && "playEntrance" in s && s.playEntrance)
          setPlayEntrance(true);
        else if (!insideArrival.current) setPhase("dialog");
        const chat = await resolveMainAssistantSession();
        if (active) setPendingSession(chat);
      })
      .catch((e) => {
        if (active) {
          setError(String(e));
          setPhase("dialog");
        }
      });
    return () => {
      active = false;
    };
  }, [retry]);
  useEffect(() => {
    if (!pendingSession || !status?.assistantCode) return;
    const controller = new AbortController();
    const chat = pendingSession;
    void (async () => {
      try {
        if (status.state !== "Ready") {
          await awaitAwakeningRun({
            sessionCode: chat.sessionCode,
            readHistory: (signal) => adapter.loadHistory!(chat.sessionCode, { limit: 20, cursor: null }, signal),
            readEvents: (signal) => listAssistantEvents(status.assistantCode, signal),
            signal: controller.signal,
            timeoutMessage: t("awakening.startTimeout"),
          });
        }
        if (!controller.signal.aborted) setSession(chat);
      } catch (e) {
        if (!controller.signal.aborted) setError(String(e));
      }
    })();
    return () => controller.abort();
  }, [pendingSession, adapter, status?.assistantCode, status?.state, retry, t]);
  useEffect(() => {
    let disposed = false;
    const id = setInterval(() => {
      if (document.hidden) return;
      void getAwakening()
        .then((s) => {
          if (!disposed) {
            setStatus(s);
            if (s.state === "Ready")
              void queryClient.invalidateQueries({ queryKey: ["assistants"] });
          }
        })
        .catch(() => {});
    }, 2000);
    return () => {
      disposed = true;
      clearInterval(id);
    };
  }, [queryClient]);
  useEffect(() => {
    if (!manual || !status) return;
    let active = true;
    void getAssistantSoul(status.assistantCode)
      .then((s) => {
        if (active) {
          setName(s.nameConfirmed ? s.name : "");
          setMarkdown(s.markdown);
        }
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [manual, status?.assistantCode]);
  const send = useCallback(
    (
      chat: ChatSession,
      content: string,
      attachments: NonNullable<SendChatMessageInput["attachments"]>
    ): SendChatMessageInput => ({
      agentCode: chat.agentCode,
      content,
      attachments,
      workspaceCode: null,
      modelSelection: { kind: "Auto" },
      clientCapabilities: { userInputRequests: true },
    }),
    []
  );
  const exitTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (exitTimer.current) clearTimeout(exitTimer.current);
    },
    []
  );
  const leave = useCallback(async () => {
    if (saving || leaving.current) return;
    leaving.current = true;
    setSaving(true);
    setError(null);
    try {
      if (status?.state !== "Ready") await deferAwakening();
      await queryClient.invalidateQueries({ queryKey: ["assistants"] });
      beginArrival(reduced);
      setPhase("exit");
      exitTimer.current = setTimeout(
        () =>
          navigate("/workspace", {
            replace: true,
          }),
        reduced ? 100 : 1000
      );
    } catch (e) {
      setError(String(e));
      setSaving(false);
      leaving.current = false;
    }
  }, [saving, status?.state, queryClient, beginArrival, reduced, navigate]);
  useEffect(() => {
    // Ready is authoritative, but saving the soul may happen before the last
    // assistant reply. Wait for history restoration and streaming to settle.
    if (status?.state !== "Ready" || phase !== "dialog" || saving || error || manual) return;
    if (!manualComplete && (!session || runState?.sessionCode !== session.sessionCode
      || runState.loading !== false || runState.active)) return;
    const timer = setTimeout(() => void leave(), 2200);
    return () => clearTimeout(timer);
  }, [status?.state, phase, saving, error, manual, manualComplete, session, runState, leave]);
  async function save() {
    if (
      !status ||
      saving ||
      !name.trim() ||
      !markdown.trim() ||
      new TextEncoder().encode(markdown).length > 16384
    )
      return;
    setSaving(true);
    setError(null);
    try {
      await saveAssistantSoul(status.assistantCode, { name, markdown });
      setStatus(await getAwakening());
      await queryClient.invalidateQueries({ queryKey: ["assistants"] });
      setManualComplete(true);
      setManual(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <main className="awakening-world" data-phase={phase} aria-label={t("awakening.title")}>
      {phase === "approach" && <BlackHole diving={playEntrance} />}
      <Button
        className="awakening-skip"
        variant="ghost"
        disabled={saving}
        onClick={() => void leave()}
      >
        {t("awakening.skip")}
      </Button>
      <div className="awakening-stage" inert={phase !== "dialog"}>
        <header className="awakening-heading">
          <div className="awakening-avatar">
            <AgentAvatar
              avatarId={avatarId}
              alt={status?.name ?? t("awakening.title")}
              className="awakening-avatar-image rounded-full"
            />
          </div>
        </header>
        {error && (
          <div role="alert" className="awakening-error">
            {error}{" "}
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                opening.current = null;
                setPendingSession(null);
                setSession(null);
                setRetry((v) => v + 1);
              }}
            >
              {t("assistant.retry")}
            </Button>
          </div>
        )}
        {manual ? (
          <section className="awakening-manual">
            <SoulFields
              name={name}
              markdown={markdown}
              state={status?.state ?? "Uninitialized"}
              disabled={saving}
              onName={setName}
              onMarkdown={setMarkdown}
              onSave={() => void save()}
            />
            <Button
              className="mt-4"
              disabled={saving || !name.trim() || !markdown.trim()}
              onClick={() => void save()}
            >
              {t("assistant.save")}
            </Button>
          </section>
        ) : (
          <section className="awakening-chat" aria-label={t("awakening.title")}>
            {session ? (
              <AgentChatWindow
                adapter={adapter}
                showHeader={false}
                showHistoryRail={false}
                autoFocusComposer={false}
                initialSession={session}
                config={{
                  agentCode: session.agentCode ?? undefined,
                  title: status?.name ?? t("awakening.title"),
                  floating: false,
                  showNewSession: false,
                  inputPlaceholder: t("assistant.messagePlaceholder"),
                }}
                createSendInput={send}
                onError={setError}
                onRunStateChange={setRunState}
              />
            ) : (
              <div role="status" className="awakening-loading">
                {t(error ? "awakening.startFailed" : "awakening.connecting")}
              </div>
            )}
          </section>
        )}
        <footer className="awakening-actions">
          <Button
            variant="ghost"
            disabled={saving}
            onClick={() => setManual((v) => !v)}
          >
            {t(manual ? "awakening.backToChat" : "awakening.manual")}
          </Button>
          {status?.state === "Ready" && (
            <Button disabled={saving} onClick={() => void leave()}>
              {t("awakening.enter")}
            </Button>
          )}
        </footer>
      </div>
    </main>
  );
}
