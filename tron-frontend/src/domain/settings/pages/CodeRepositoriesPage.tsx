import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Pencil, PlugZap, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { SelectInput } from "@/components/forms";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { SelectItem } from "@/components/ui/select";
import { useToast } from "@/components/ui/toast";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { checkConnection, createConnection, deleteConnection, listConnections, updateConnection, type ConnectorRecord } from "@/domain/project/api/connectorApi";

type Provider = "github" | "gitlab" | "gitee";
const endpoints: Record<Provider, string> = { github: "https://api.github.com", gitlab: "https://gitlab.com", gitee: "https://gitee.com" };
const providerNames: Record<Provider, string> = { github: "GitHub", gitlab: "GitLab", gitee: "Gitee" };

function Connections() {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [connections, setConnections] = useState<ConnectorRecord[]>([]);
  const [provider, setProvider] = useState<Provider>("github");
  const [name, setName] = useState("");
  const [endpoint, setEndpoint] = useState(endpoints.github);
  const [token, setToken] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    listConnections(controller.signal).then(setConnections).catch((error: unknown) => {
      if (!controller.signal.aborted) toast.error(error instanceof Error ? error.message : t("topic.loadFailed"));
    });
    return () => controller.abort();
  }, [t, toast]);

  function closeForm() { setFormOpen(false); setEditing(null); setToken(""); }
  function create() { setEditing(null); setProvider("github"); setName(""); setEndpoint(endpoints.github); setToken(""); setFormOpen(true); }
  function edit(connection: ConnectorRecord) {
    if (!(connection.providerId in endpoints)) return;
    setEditing(connection.connectionCode); setProvider(connection.providerId as Provider);
    setName(connection.name); setEndpoint(connection.endpoint); setToken(""); setFormOpen(true);
  }
  async function save() {
    if (pending || !name.trim() || !endpoint.trim() || (!editing && !token)) return;
    setPending(true);
    try {
      const input = { providerId: provider, name: name.trim(), endpoint: endpoint.trim() };
      const saved = editing ? await updateConnection(editing, { ...input, token: token || null }) : await createConnection({ ...input, token });
      setConnections((current) => [...current.filter((item) => item.connectionCode !== saved.connectionCode), saved]);
      closeForm(); toast.success(t("topic.connectionSaved"));
    } catch (error) { toast.error(error instanceof Error ? error.message : t("topic.connectFailed")); }
    finally { setPending(false); }
  }
  async function check(code: string) {
    setPending(true);
    try { await checkConnection(code); toast.success(t("topic.connected")); }
    catch (error) { toast.error(error instanceof Error ? error.message : t("topic.connectFailed")); }
    finally { setPending(false); }
  }
  async function remove(code: string) {
    setPending(true);
    try { await deleteConnection(code); setConnections((current) => current.filter((item) => item.connectionCode !== code)); setDeleting(null); }
    catch (error) { toast.error(error instanceof Error ? error.message : t("topic.connectFailed")); }
    finally { setPending(false); }
  }

  return <div className="space-y-4 overflow-auto">
    <div className="flex flex-wrap items-start justify-between gap-3"><h1 className="text-xl font-semibold">{t("topic.repositorySettings")}</h1><Button onClick={create} disabled={pending}>{t("topic.connectPlatform")}</Button></div>
    <Dialog open={formOpen} onOpenChange={(open) => { if (!open && !pending) closeForm(); }}>
      <DialogContent className="sm:max-w-lg" showCloseButton={!pending}>
        <DialogHeader><DialogTitle>{t(editing ? "topic.editConnection" : "topic.newConnection")}</DialogTitle></DialogHeader>
        <form onSubmit={(event) => { event.preventDefault(); void save(); }}>
          <fieldset disabled={pending} className="min-w-0 space-y-4">
            <Field>
              <FieldLabel htmlFor="repository-connection-provider">{t("topic.connectionType")}</FieldLabel>
              <SelectInput id="repository-connection-provider" value={provider} disabled={Boolean(editing) || pending} onValueChange={(value) => {
                if (value !== "github" && value !== "gitlab" && value !== "gitee") return;
                setProvider(value); setEndpoint(endpoints[value]); setToken("");
              }}>
                {(["github", "gitlab", "gitee"] as const).map((value) => <SelectItem key={value} value={value}>{providerNames[value]}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field><FieldLabel htmlFor="repository-connection-name">{t("topic.connectionName")}</FieldLabel><Input id="repository-connection-name" required value={name} onChange={(event) => setName(event.target.value)} /></Field>
            <Field><FieldLabel htmlFor="repository-connection-endpoint">{t("topic.endpoint")}</FieldLabel><Input id="repository-connection-endpoint" required value={endpoint} onChange={(event) => setEndpoint(event.target.value)} /></Field>
            <Field><FieldLabel htmlFor="repository-connection-token">Token</FieldLabel><Input id="repository-connection-token" type="password" required={!editing} value={token} onChange={(event) => setToken(event.target.value)} autoComplete="off" placeholder={editing ? t("topic.keepToken") : ""} /></Field>
          </fieldset>
          <DialogFooter className="mt-6">
            <Button type="button" variant="outline" disabled={pending} onClick={closeForm}>{t("common:dialogSelect.cancel")}</Button>
            <Button type="submit" disabled={pending || !name.trim() || !endpoint.trim() || (!editing && !token)}>{t(editing ? "topic.saveConnection" : "topic.connect")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
    {connections.length === 0 ? <p className="text-sm text-muted-foreground">{t("topic.noConnections")}</p> : <div className="divide-y divide-border">{connections.map((connection) => <div key={connection.connectionCode} className="space-y-2 py-4"><div className="flex flex-wrap items-center justify-between gap-2"><div className="min-w-0"><p className="font-medium">{connection.name}</p><p className="break-all text-sm text-muted-foreground">{connection.providerId} · {connection.endpoint}</p></div><div className="flex shrink-0 items-center gap-1">
      <Tooltip><TooltipTrigger render={<Button size="icon" variant="ghost" disabled={pending} aria-label={`${t("topic.testConnection")} ${connection.name}`} onClick={() => void check(connection.connectionCode)} />}><PlugZap aria-hidden="true" /></TooltipTrigger><TooltipContent>{t("topic.testConnection")}</TooltipContent></Tooltip>
      <Tooltip><TooltipTrigger render={<Button size="icon" variant="ghost" disabled={pending} aria-label={`${t("topic.editConnection")} ${connection.name}`} onClick={() => edit(connection)} />}><Pencil aria-hidden="true" /></TooltipTrigger><TooltipContent>{t("topic.editConnection")}</TooltipContent></Tooltip>
      <Tooltip><TooltipTrigger render={<Button size="icon" variant="ghost" className="text-destructive hover:text-destructive" disabled={pending} aria-label={`${t("topic.deleteConnection")} ${connection.name}`} onClick={() => setDeleting(connection.connectionCode)} />}><Trash2 aria-hidden="true" /></TooltipTrigger><TooltipContent>{t("topic.deleteConnection")}</TooltipContent></Tooltip>
    </div></div>{deleting === connection.connectionCode ? <div className="space-y-2"><p className="text-sm">{t("topic.deleteConnectionDescription")}</p><div className="flex gap-2"><Button size="sm" variant="outline" onClick={() => setDeleting(null)}>{t("topic.close")}</Button><Button size="sm" variant="destructive" disabled={pending} onClick={() => void remove(connection.connectionCode)}>{t("topic.deleteConnection")}</Button></div></div> : null}</div>)}</div>}
  </div>;
}

export function CodeRepositoriesPage() {
  return <Connections />;
}
