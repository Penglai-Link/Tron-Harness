import { useEffect, useMemo, useRef, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";
import { isApiError } from "@/shared/api/httpClient";
import { useAuthStore } from "@/store/authStore";
import { getTopicMemo, saveTopicNotes, type TopicMemo } from "../api/topicApi";

type SavePhase = "saved" | "waiting" | "saving" | "error" | "conflict";
interface NotesState {
  memo: TopicMemo | null;
  notes: string;
  phase: SavePhase;
  loadError: unknown;
}
interface Draft { notes: string; baseNotes: string; conflict: boolean; owner: string }

/** One serialized writer per mounted topic. Server revisions protect other windows. */
class TopicNotesEditor {
  private state: NotesState = { memo: null, notes: "", phase: "saved", loadError: null };
  private listeners = new Set<() => void>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private running: Promise<boolean> | null = null;
  private attempt: string | null = null;
  private active = false;
  private readGeneration = 0;
  private restored = false;
  private owner = crypto.randomUUID();
  private storageKey: string;

  constructor(private projectCode: string, private userCode: string, private notify: (key: string) => void) {
    this.storageKey = `tron:topic-notes:${encodeURIComponent(userCode)}:${projectCode}`;
  }

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };
  getSnapshot = () => this.state;
  getMemo = () => this.state.memo;
  private sameUser = () => useAuthStore.getState().session?.user.userCode === this.userCode;

  private readDraft(): Draft | null {
    try {
      const value: unknown = JSON.parse(sessionStorage.getItem(this.storageKey) ?? "null");
      if (value && typeof value === "object" && "notes" in value && typeof value.notes === "string"
        && "baseNotes" in value && typeof value.baseNotes === "string"
        && "owner" in value && typeof value.owner === "string") {
        return { notes: value.notes, baseNotes: value.baseNotes, owner: value.owner, conflict: "conflict" in value && value.conflict === true };
      }
    } catch { /* Saving still works if browser draft storage is unavailable. */ }
    return null;
  }

  private publish(next: Partial<NotesState>) {
    this.state = { ...this.state, ...next };
    const { memo, notes, phase } = this.state;
    try {
      const stored = this.readDraft();
      // A newly opened editor owns its restored draft; late replies cannot erase it.
      if (memo && (!stored || stored.owner === this.owner)) {
        if (notes === memo.notes) sessionStorage.removeItem(this.storageKey);
        else sessionStorage.setItem(this.storageKey, JSON.stringify({ notes, baseNotes: memo.notes, conflict: phase === "conflict", owner: this.owner } satisfies Draft));
      }
    } catch { /* Keep the in-memory draft and unload protection. */ }
    this.listeners.forEach((listener) => listener());
  }

  acceptMemo = (next: TopicMemo) => {
    if (next.projectCode !== this.projectCode || !this.sameUser()) return;
    const previous = this.state.memo;
    if (previous && next.revision < previous.revision) return;
    if (!this.restored) {
      this.restored = true;
      const draft = this.readDraft();
      if (draft && draft.notes !== next.notes) {
        try { sessionStorage.setItem(this.storageKey, JSON.stringify({ ...draft, owner: this.owner })); } catch { /* Optional recovery. */ }
        this.publish({ memo: next, notes: draft.notes, phase: draft.conflict || draft.baseNotes !== next.notes ? "conflict" : "waiting", loadError: null });
        this.schedule();
        return;
      }
      // Adopt a clean recovered entry before removing it.
      if (draft) { try { sessionStorage.removeItem(this.storageKey); } catch { /* Optional recovery. */ } }
    }
    const dirty = previous !== null && this.state.notes !== previous.notes;
    const notes = dirty ? this.state.notes : next.notes;
    const conflict = dirty && (this.state.phase === "conflict" || (next.notes !== previous.notes && next.notes !== notes && next.notes !== this.attempt));
    this.publish({ memo: next, notes, loadError: null, phase: conflict ? "conflict" : this.running ? "saving" : notes === next.notes ? "saved" : "waiting" });
    this.schedule();
  };

  refresh = async () => {
    const generation = ++this.readGeneration;
    try {
      const next = await getTopicMemo(this.projectCode);
      if (!this.active || generation !== this.readGeneration || !this.sameUser()) return false;
      this.acceptMemo(next);
      return true;
    } catch (error) {
      if (this.active && generation === this.readGeneration && this.sameUser()) {
        if (!this.state.memo) this.publish({ loadError: error });
        else this.notify("topic.loadFailed");
      }
      return false;
    }
  };

  start = () => { this.active = true; void this.refresh(); };
  stop = () => {
    this.active = false;
    ++this.readGeneration;
    clearTimeout(this.timer);
    // Do not abort a write when a tab closes or navigation replaces the page.
    void this.flush();
  };

  private schedule() {
    clearTimeout(this.timer);
    if (this.active && this.state.phase === "waiting") this.timer = setTimeout(() => { void this.flush(); }, 600);
  }

  setNotes = (notes: string) => {
    const conflict = this.state.phase === "conflict" && notes !== this.state.memo?.notes;
    this.publish({ notes, phase: conflict ? "conflict" : this.running ? "saving" : notes === this.state.memo?.notes ? "saved" : "waiting" });
    this.schedule();
  };

  flush = (): Promise<boolean> => {
    clearTimeout(this.timer);
    if (this.running) return this.running;
    if (!this.state.memo || !this.sameUser() || this.state.phase === "conflict" || this.state.phase === "error") return Promise.resolve(false);
    if (this.state.notes === this.state.memo.notes) return Promise.resolve(true);
    this.running = this.drain().finally(() => { this.running = null; this.attempt = null; });
    return this.running;
  };

  private async drain() {
    let rebases = 0;
    while (this.sameUser() && this.state.memo && this.state.notes !== this.state.memo.notes && this.state.phase !== "conflict") {
      const baseline = this.state.memo;
      const submitted = this.state.notes;
      if (new TextEncoder().encode(submitted).length > 64 * 1024) {
        this.publish({ phase: "error" });
        if (this.active) this.notify("topic.autosave.tooLong");
        return false;
      }
      this.attempt = submitted;
      this.publish({ phase: "saving" });
      try {
        const next = await saveTopicNotes(this.projectCode, baseline.revision, submitted);
        if (!this.sameUser()) return false;
        // Acknowledgements advance the baseline, never replace newer typed text.
        if (next.revision >= (this.state.memo?.revision ?? 0)) this.publish({ memo: next });
        if (this.getSnapshot().phase !== "conflict") this.publish({ phase: this.state.notes === this.state.memo?.notes ? "saved" : "saving" });
      } catch (error) {
        if (!this.sameUser()) return false;
        if (isApiError(error) && error.code === "TOPIC_REVISION_CONFLICT" && rebases++ < 3) {
          try {
            const fetched = await getTopicMemo(this.projectCode);
            if (!this.sameUser()) return false;
            const latest = fetched.revision >= (this.state.memo?.revision ?? 0) ? fetched : this.state.memo!;
            if (latest.notes === baseline.notes || latest.notes === submitted) {
              this.publish({ memo: latest, phase: this.state.notes === latest.notes ? "saved" : "saving" });
              continue; // Only references changed, or the exact notes were already saved.
            }
            this.publish({ memo: latest, phase: "conflict" });
            return false;
          } catch { /* Preserve the draft and offer an explicit retry. */ }
        }
        this.publish({ phase: "error" });
        if (this.active) this.notify("topic.autosave.failed");
        return false;
      }
    }
    return this.sameUser() && this.state.phase !== "conflict" && this.state.notes === this.state.memo?.notes;
  }

  retry = () => { if (this.state.phase === "error") this.publish({ phase: "waiting" }); return this.flush(); };
  keepMine = () => { this.publish({ phase: "waiting" }); return this.flush(); };
  useLatest = () => { if (this.state.memo) this.publish({ notes: this.state.memo.notes, phase: "saved" }); };
  protectUnload = (event: BeforeUnloadEvent) => {
    if (this.state.memo && (this.running || this.state.notes !== this.state.memo.notes)) {
      event.preventDefault();
      event.returnValue = "";
      void this.flush();
    }
  };
}

export function useTopicNotes(projectCode: string, userCode: string) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const notify = useRef((key: string) => toast.error(t(key)));
  notify.current = (key) => toast.error(t(key));
  const editor = useMemo(() => new TopicNotesEditor(projectCode, userCode, (key) => notify.current(key)), [projectCode, userCode]);
  const state = useSyncExternalStore(editor.subscribe, editor.getSnapshot, editor.getSnapshot);
  useEffect(() => {
    editor.start();
    const focus = () => { void editor.refresh(); };
    window.addEventListener("focus", focus);
    window.addEventListener("beforeunload", editor.protectUnload);
    return () => {
      window.removeEventListener("focus", focus);
      window.removeEventListener("beforeunload", editor.protectUnload);
      editor.stop();
    };
  }, [editor]);
  return { ...state, editor };
}
