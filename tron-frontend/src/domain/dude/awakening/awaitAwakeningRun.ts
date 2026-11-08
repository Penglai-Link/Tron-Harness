import type { SessionHistory } from "@personai/agent-ui-chat-core";
import type { DudeEvent } from "../api/dudeApi";

/** Bridge the queued event to Agent UI's native run observer. An empty session
 * has no active run, so Agent UI correctly does not poll it on its own. */
export async function awaitAwakeningRun({
  sessionCode, readHistory, readEvents, signal, timeoutMessage,
}: {
  sessionCode: string;
  readHistory: (signal: AbortSignal) => Promise<SessionHistory>;
  readEvents: (signal: AbortSignal) => Promise<DudeEvent[]>;
  signal: AbortSignal;
  timeoutMessage: string;
}): Promise<void> {
  const controller = new AbortController();
  const abort = () => controller.abort(signal.reason);
  signal.addEventListener("abort", abort, { once: true });
  if (signal.aborted) abort();
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, 45000);
  const pending = controller.signal;
  try {
    while (true) {
      pending.throwIfAborted();
      const [history, events] = await Promise.all([readHistory(pending), readEvents(pending)]);
      pending.throwIfAborted();
      // Model routing can fail before a Framework run is created. Surface the
      // real event failure rather than leaving an empty chat forever.
      const latest = events.filter((event) => event.sessionCode === sessionCode && event.source === "UserMessage")
        .sort((a, b) => b.createdAt - a.createdAt)[0];
      if (latest?.status === "Failed") throw new Error(latest.result || timeoutMessage);
      // A retry may coexist with an older failed run. Do not hand off that stale
      // run while the new event is still queued, or idle polling stops again.
      const startsAfter = latest && ["Queued", "Running"].includes(latest.status) ? latest.createdAt : 0;
      if ([...history.activeRuns, ...history.runs].some((run) => run.startedAtMs >= startsAfter)
        || (!startsAfter && (history.pendingInteractions.length
          || history.records.messages.some((message) => message.role === "Assistant")))) return;
      await new Promise<void>((resolve, reject) => {
        const stop = () => { clearTimeout(timer); reject(pending.reason); };
        const timer = setTimeout(() => { pending.removeEventListener("abort", stop); resolve(); }, 750);
        pending.addEventListener("abort", stop, { once: true });
        if (pending.aborted) stop();
      });
    }
  } catch (error) {
    if (timedOut) throw new Error(timeoutMessage);
    throw error;
  } finally {
    clearTimeout(timeout);
    signal.removeEventListener("abort", abort);
  }
}
