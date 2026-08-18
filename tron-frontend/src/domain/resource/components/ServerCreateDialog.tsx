import {
  useEffect,
  useState,
  type FormEvent,
} from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { toastManager } from "@/components/ui/toast";
import { createServerConnection } from "@/domain/resource/api";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  ConnectionTextareaField,
  ConnectionTextField,
} from "./ConnectionFormFields";
import { generateConnectionResourceCode } from "./connectionResourceCode";
import {
  emptyServerAccount,
  ServerAccountFields,
  type ServerAccountFormValue,
} from "./ServerAccountFields";

interface ServerCreateDialogProps {
  onCreated: () => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  workspaceCode: string;
}

export function ServerCreateDialog({ onCreated, onOpenChange, open, workspaceCode }: ServerCreateDialogProps) {
  const { t } = useTranslation("pages");
  const [serverCode, setServerCode] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("22");
  const [account, setAccount] = useState<ServerAccountFormValue>({ ...emptyServerAccount });
  const [submitting, setSubmitting] = useState(false);
  const pageT = (key: string) => t(`resourceConnections.${key}`);

  useEffect(() => {
    if (!open) return;
    setServerCode(generateConnectionResourceCode("SERVER"));
    setAccount({ ...emptyServerAccount, accountCode: generateConnectionResourceCode("SERVER_ACCOUNT") });
  }, [open]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const request = {
        allowInheritance: false,
        description: description.trim() || null,
        host: host.trim(),
        initialAccount: {
          accountCode: account.accountCode.trim(),
          allowInheritance: false,
          authenticationType: account.authenticationType,
          description: account.description.trim() || null,
          name: account.name.trim(),
          ...(account.authenticationType === "Password"
            ? { passwordVariableKey: account.passwordVariableKey.trim() }
            : {
                privateKeyVariableKey: account.privateKeyVariableKey.trim(),
                ...(account.privateKeyPassphraseVariableKey.trim()
                  ? { privateKeyPassphraseVariableKey: account.privateKeyPassphraseVariableKey.trim() }
                  : {}),
              }),
          username: account.username.trim(),
        },
        name: name.trim(),
        port: Number(port),
        scopeKind: "WORKSPACE" as const,
        serverCode: serverCode.trim(),
        workspaceCode,
      };
      const connectivityVerified = getServiceDataSource("resource") === "api"
        ? (await createServerConnection(request)).connectivityVerified
        : true;
      toastManager.add({
        description: pageT("messages.serverCreated"),
        title: connectivityVerified
          ? pageT("messages.connectionVerified")
          : pageT("messages.created"),
        type: "success",
      });
      onOpenChange(false);
      onCreated();
    } catch (cause) {
      toastManager.add({
        description: cause instanceof Error ? cause.message : pageT("messages.createFailed"),
        title: pageT("messages.connectionFailed"),
        type: "error",
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-h-[88vh] overflow-y-auto sm:max-w-2xl">
        <form onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>{pageT("server.createTitle")}</DialogTitle>
          </DialogHeader>
          <div className="grid gap-4 py-5 sm:grid-cols-2">
            <ConnectionTextField id="server-code" label={pageT("fields.serverCode")} onChange={setServerCode} readOnly required value={serverCode} />
            <ConnectionTextField id="server-name" label={pageT("fields.name")} onChange={setName} placeholder={pageT("placeholders.serverName")} required value={name} />
            <ConnectionTextField id="server-host" label={pageT("fields.host")} onChange={setHost} placeholder="10.20.0.12" required value={host} />
            <ConnectionTextField id="server-port" label={pageT("fields.port")} min={1} onChange={setPort} required type="number" value={port} />
            <div className="sm:col-span-2">
              <ConnectionTextareaField id="server-description" label={pageT("fields.description")} onChange={setDescription} value={description} />
            </div>
            <div className="sm:col-span-2 border-t pt-4">
              <h3 className="font-medium">{pageT("server.initialAccount")}</h3>
            </div>
            <ServerAccountFields idPrefix="initial-account" onChange={setAccount} value={account} />
          </div>
          <DialogFooter>
            <Button disabled={submitting} type="button" variant="outline" onClick={() => onOpenChange(false)}>{pageT("actions.cancel")}</Button>
            <Button disabled={submitting} type="submit">
              {submitting ? <Spinner /> : null}
              {submitting ? pageT("actions.verifying") : pageT("actions.createAndVerify")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
