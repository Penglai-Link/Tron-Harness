import { useEffect, useRef, useState } from "react";
import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { InfoTooltip } from "@/components/info-tooltip";
import { useToast } from "@/components/ui/toast";
import { chooseSystemDirectoryPath } from "@/domain/directory/api/systemDirectoryPicker";
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
  const [picking, setPicking] = useState(false);
  const pickingRef = useRef(false);

  useEffect(() => {
    if (!reference) return;
    const controller = new AbortController();
    setOptions(null); setError(""); setLoading(true);
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

  async function chooseDirectory() {
    if (pickingRef.current || !options) return;
    pickingRef.current = true;
    setPicking(true);
    try {
      const selected = await chooseSystemDirectoryPath(workspaceCode);
      if (!selected) return;
      if (mode === "Clone") {
        const folderName = options.suggestedPath.split(/[\\/]/).filter(Boolean).pop();
        const separator = selected.includes("\\") ? "\\" : "/";
        setPath(folderName ? `${selected.replace(/[\\/]+$/, "")}${separator}${folderName}` : selected);
      } else {
        setPath(selected);
      }
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("chatSidebar.directory.browseFailed"));
    } finally { pickingRef.current = false; setPicking(false); }
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

  return <Dialog open={reference !== null} onOpenChange={(open) => { if (!open && !pending && !picking) onClose(); }}>
    <DialogContent className="sm:max-w-xl" showCloseButton={!pending && !picking}>
      <DialogHeader><DialogTitle>{t("topic.checkout.title")}</DialogTitle></DialogHeader>
      <p className="text-sm text-muted-foreground">{reference?.label}</p>
      {loading ? <p>{t("chatSidebar.loading")}</p> : null}
      {error ? <p role="alert" className="text-sm text-destructive">{error}</p> : null}
      {options ? <fieldset disabled={pending || picking} className="min-w-0 space-y-4">
        <div className="flex gap-2">
          <Button type="button" variant={mode === "Existing" ? "secondary" : "ghost"} onClick={() => { setMode("Existing"); setPath(options.existing[0]?.path ?? ""); }}>{t("topic.checkout.existing")}</Button>
          <Button type="button" disabled={!options.cloneSupported} variant={mode === "Clone" ? "secondary" : "ghost"} onClick={() => { setMode("Clone"); setPath(options.suggestedPath); }}>{t("topic.checkout.clone")}</Button>
        </div>
        {!options.cloneSupported ? <p className="text-sm text-muted-foreground">{t("topic.checkout.httpsRequired")}</p> : null}
        <Field>
          <div className="flex items-center gap-2">
            <FieldLabel htmlFor="checkout-path">{mode === "Existing" ? t("topic.checkout.existingPath") : t("topic.checkout.clonePath")}</FieldLabel>
            <InfoTooltip ariaLabel={t("topic.checkout.pathHelp")}>{t(mode === "Existing" ? "topic.checkout.existingHint" : "topic.checkout.clonePathHint")}</InfoTooltip>
          </div>
          <div className="flex w-full items-center gap-2">
            <Input id="checkout-path" className="min-w-0 flex-1 font-mono" value={path} onChange={(event) => setPath(event.target.value)} />
            <Button type="button" variant="outline" className="shrink-0" disabled={picking} onClick={() => void chooseDirectory()}><FolderOpen size={16} />{t(mode === "Clone" ? "topic.checkout.chooseParent" : "topic.checkout.chooseFolder")}</Button>
          </div>
        </Field>
        {mode === "Clone" ? <Field>
          <div className="flex items-center gap-2"><FieldLabel htmlFor="checkout-username">{t("topic.checkout.username")}</FieldLabel><InfoTooltip ariaLabel={t("topic.checkout.usernameHelp")}>{t(options.providerId === "gitee" ? "topic.checkout.giteeUsernameHint" : "topic.checkout.usernameHint")}</InfoTooltip></div>
          <Input id="checkout-username" value={username} onChange={(event) => setUsername(event.target.value)} />
        </Field> : null}
      </fieldset> : null}
      <DialogFooter><Button variant="outline" disabled={pending || picking} onClick={onClose}>{t("topic.close")}</Button><Button disabled={!options || loading || pending || picking || !path.trim() || (mode === "Clone" && !username.trim())} onClick={() => void submit()}>{pending ? t("topic.checkout.preparing") : t("topic.checkout.prepare")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
