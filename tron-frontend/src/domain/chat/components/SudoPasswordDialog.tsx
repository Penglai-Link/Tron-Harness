import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { requestApi } from "@/shared/api/httpClient";

interface SudoInput {
  requestId: string;
  command: string;
  args: string[];
  cwd: string | null;
}

export function SudoPasswordDialog({ sessionCode, workspaceCode, dudeCode, knowledgeBaseId }: {
  sessionCode: string; workspaceCode: string; dudeCode?: string; knowledgeBaseId?: string;
}) {
  const headers: Record<string, string> = { "X-Tron-Sudo-Input": "1" };
  if (dudeCode) headers["X-Dude-Code"] = dudeCode;
  if (knowledgeBaseId) headers["X-Knowledge-Base-Code"] = knowledgeBaseId;
  const { t } = useTranslation("pages");
  const [pending, setPending] = useState<SudoInput | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const passwordRef = useRef<HTMLInputElement>(null);
  const submittingRef = useRef(false);
  const dismissedRef = useRef(new Set<string>());
  useEffect(() => {
    let disposed = false;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const inputs = await requestApi<SudoInput[]>("GET /api/v1/chats/sessions/{sessionCode}/sudo-inputs", {
          pathParams: { sessionCode }, queryParams: { workspaceCode }, headers,
          cache: "no-store", signal: AbortSignal.any([controller.signal, AbortSignal.timeout(5000)]), retryOnUnauthorized: false,
        });
        if (!disposed && !submittingRef.current) {
          setPending((previous) => {
            const next = inputs.find((input) => !dismissedRef.current.has(input.requestId)) ?? null;
            if (next?.requestId !== previous?.requestId) {
              if (passwordRef.current) passwordRef.current.value = "";
              setError(false);
            }
            return next;
          });
        }
      } catch {
        if (!disposed && !submittingRef.current) {
          if (passwordRef.current) passwordRef.current.value = "";
          setPending(null);
        }
      }
      if (!disposed) timer = setTimeout(() => { void poll(); }, 1000);
    }
    void poll();
    return () => { disposed = true; controller.abort(); clearTimeout(timer); };
  }, [sessionCode, workspaceCode, dudeCode, knowledgeBaseId]);

  async function answer(cancel: boolean) {
    if (!pending || submittingRef.current) return;
    const value = cancel ? null : passwordRef.current?.value ?? "";
    if (value !== null && (!value || new TextEncoder().encode(value).length > 4096 || /[\r\n\0]/.test(value))) {
      setError(true);
      return;
    }
    const id = pending.requestId;
    submittingRef.current = true;
    setBusy(true);
    setError(false);
    // The input is uncontrolled and wiped before sending; never use chat state or storage.
    if (passwordRef.current) passwordRef.current.value = "";
    try {
      await requestApi("POST /api/v1/chats/sessions/{sessionCode}/sudo-inputs/{requestId}/answer", {
        pathParams: { sessionCode, requestId: id }, queryParams: { workspaceCode }, headers,
        body: { password: value }, retryOnUnauthorized: false,
        signal: AbortSignal.timeout(10000), cache: "no-store",
      });
      dismissedRef.current.add(id);
      setPending(null);
    } catch { setError(true); }
    finally { submittingRef.current = false; setBusy(false); }
  }

  return <Dialog open={pending !== null} onOpenChange={(open) => { if (!open) void answer(true); }}>
    <DialogContent showCloseButton={false}>
      <DialogHeader>
        <DialogTitle>{t("chat.sudo.title")}</DialogTitle>
        <DialogDescription>{t("chat.sudo.description")}</DialogDescription>
      </DialogHeader>
      <pre className="max-h-32 overflow-auto whitespace-pre-wrap break-all rounded bg-muted p-2 text-xs">{pending ? [pending.command, ...pending.args].map((arg) => JSON.stringify(arg)).join(" ") : ""}</pre>
      {pending?.cwd && <p className="break-all text-xs text-muted-foreground">{pending.cwd}</p>}
      <form onSubmit={(event) => { event.preventDefault(); void answer(false); }} className="grid gap-4">
        <div className="grid gap-2">
          <Label htmlFor={`sudo-password-${pending?.requestId}`}>{t("chat.sudo.password")}</Label>
          <Input key={pending?.requestId} id={`sudo-password-${pending?.requestId}`} ref={passwordRef} type="password" autoComplete="off" autoFocus maxLength={4096} disabled={busy} aria-invalid={error} aria-describedby={error ? "sudo-input-error" : undefined} />
          {error && <p id="sudo-input-error" role="alert" className="text-sm text-destructive">{t("chat.sudo.failed")}</p>}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" disabled={busy} onClick={() => { void answer(true); }}>{t("chat.sudo.cancel")}</Button>
          <Button type="submit" disabled={busy}>{t("chat.sudo.submit")}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>;
}
