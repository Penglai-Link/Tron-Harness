import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import { browseDirectories, type DirectoryBrowserPage } from "@/domain/directory/api/directoryApi";
import { getCheckoutOptions, prepareCheckout, type CheckoutOptions, type TopicMemo, type TopicReference } from "../api/topicApi";

export function TopicCheckoutDialog({ projectCode, workspaceCode, reference, onClose, onPrepared }: {
  projectCode: string;
  workspaceCode: string;
  reference: TopicReference | null;
  onClose: () => void;
  onPrepared: (memo: TopicMemo) => void;
}) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [options, setOptions] = useState<CheckoutOptions | null>(null);
  const [mode, setMode] = useState<"Existing" | "Clone">("Clone");
  const [path, setPath] = useState("");
  const [username, setUsername] = useState("");
  const [loading, setLoading] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [browserPage, setBrowserPage] = useState<DirectoryBrowserPage | null>(null);
  const [browseError, setBrowseError] = useState("");
  const [browsing, setBrowsing] = useState(false);

  useEffect(() => {
    if (!reference) return;
    const controller = new AbortController();
    setOptions(null); setError(""); setBrowserPage(null); setBrowseError(""); setLoading(true);
    getCheckoutOptions(projectCode, reference.referenceCode, controller.signal).then((value) => {
      if (controller.signal.aborted) return;
      setOptions(value);
      setMode(value.existing.length || !value.cloneSupported ? "Existing" : "Clone");
      setPath(value.existing[0]?.path ?? (value.cloneSupported ? value.suggestedPath : ""));
      setUsername(value.providerId === "gitee" ? "" : "oauth2");
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("topic.checkout.failed"));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [projectCode, reference, t]);

  async function browse(value: string | null) {
    setBrowsing(true); setBrowseError("");
    try {
      const page = await browseDirectories(workspaceCode, value);
      setBrowserPage(page); setPath(page.path);
    } catch (reason) {
      setBrowseError(reason instanceof Error ? reason.message : t("chatSidebar.directory.browseFailed"));
    } finally { setBrowsing(false); }
  }

  async function submit() {
    if (!reference || !path.trim() || pending) return;
    setPending(true);
    try {
      const input = mode === "Existing" ? { mode, path: path.trim() } : { mode, path: path.trim(), username: username.trim() };
      const memo = await prepareCheckout(projectCode, reference.referenceCode, input);
      window.dispatchEvent(new Event("tron:directories-changed"));
      onPrepared(memo); onClose();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("topic.checkout.failed"));
    } finally { setPending(false); }
  }

  return <Dialog open={reference !== null} onOpenChange={(open) => { if (!open && !pending) onClose(); }}>
    <DialogContent>
      <DialogHeader><DialogTitle>{t("topic.checkout.title")}</DialogTitle></DialogHeader>
      <p className="text-sm text-muted-foreground">{reference?.label}</p>
      {loading ? <p>{t("chatSidebar.loading")}</p> : null}
      {error ? <p role="alert" className="text-sm text-destructive">{error}</p> : null}
      {options ? <div className="space-y-3">
        <div className="flex gap-2">
          <Button type="button" variant={mode === "Existing" ? "secondary" : "ghost"} onClick={() => { setMode("Existing"); setPath(options.existing[0]?.path ?? ""); }}>{t("topic.checkout.existing")}</Button>
          <Button type="button" disabled={!options.cloneSupported} variant={mode === "Clone" ? "secondary" : "ghost"} onClick={() => { setMode("Clone"); setPath(options.suggestedPath); }}>{t("topic.checkout.clone")}</Button>
        </div>
        {!options.cloneSupported ? <p className="text-sm text-muted-foreground">{t("topic.checkout.httpsRequired")}</p> : null}
        {mode === "Existing" && options.existing.length ? <div className="space-y-1">{options.existing.map((directory) => <Button key={directory.directoryCode} type="button" variant="ghost" className="h-auto w-full justify-start truncate" onClick={() => setPath(directory.path)}>{directory.name} · {directory.path}</Button>)}</div> : null}
        <Field><FieldLabel htmlFor="checkout-path">{mode === "Existing" ? t("topic.checkout.existingPath") : t("topic.checkout.clonePath")}</FieldLabel><Input id="checkout-path" value={path} onChange={(event) => setPath(event.target.value)} /></Field>
        {mode === "Existing" ? <>
          <div className="flex gap-2"><Button type="button" variant="outline" disabled={browsing} onClick={() => void browse(path.trim() || null)}>{t("chatSidebar.directory.browse")}</Button><Button type="button" variant="outline" disabled={browsing || !browserPage?.parent} onClick={() => void browse(browserPage?.parent ?? null)}>{t("chatSidebar.directory.parent")}</Button></div>
          {browseError ? <p role="alert" className="text-sm text-destructive">{browseError}</p> : null}
          {browserPage ? <div className="max-h-40 overflow-y-auto rounded border border-border" aria-label={t("chatSidebar.directory.folders")}>{browserPage.entries.map((entry) => <Button key={entry.path} type="button" variant="ghost" className="h-auto w-full justify-start" onClick={() => void browse(entry.path)}>{entry.name}</Button>)}</div> : null}
        </> : null}
        {mode === "Clone" ? <><Field><FieldLabel htmlFor="checkout-username">{t("topic.checkout.username")}</FieldLabel><Input id="checkout-username" value={username} onChange={(event) => setUsername(event.target.value)} /></Field><p className="text-sm text-muted-foreground">{t(options.providerId === "gitee" ? "topic.checkout.giteeHint" : "topic.checkout.cloneHint")}</p></> : <p className="text-sm text-muted-foreground">{t("topic.checkout.existingHint")}</p>}
      </div> : null}
      <DialogFooter><Button variant="outline" disabled={pending} onClick={onClose}>{t("topic.close")}</Button><Button disabled={!options || loading || pending || !path.trim() || (mode === "Clone" && !username.trim())} onClick={() => void submit()}>{pending ? t("topic.checkout.preparing") : t("topic.checkout.prepare")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
