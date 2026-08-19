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
import { createServerAccountConnection } from "@/domain/resource/api";
import type { ManagedResourceRecord } from "@/domain/resource/hooks";
import { createMockManagedResource } from "@/domain/resource/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  ConnectionSelectField,
  ConnectionSelectItem,
} from "./ConnectionFormFields";
import { generateConnectionResourceCode } from "./connectionResourceCode";
import {
  emptyServerAccount,
  ServerAccountFields,
  type ServerAccountFormValue,
} from "./ServerAccountFields";

interface ServerAccountCreateDialogProps {
  onCreated: () => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  servers: ManagedResourceRecord[];
  workspaceCode: string;
}

export function ServerAccountCreateDialog({
  onCreated,
  onOpenChange,
  open,
  servers,
  workspaceCode,
}: ServerAccountCreateDialogProps) {
  const { t } = useTranslation("pages");
  const [serverCode, setServerCode] = useState("");
  const [account, setAccount] = useState<ServerAccountFormValue>({ ...emptyServerAccount });
  const [submitting, setSubmitting] = useState(false);
  const pageT = (key: string) => t(`resourceConnections.${key}`);

  useEffect(() => {
    if (open) setAccount({ ...emptyServerAccount, accountCode: generateConnectionResourceCode("SERVER_ACCOUNT") });
  }, [open]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const accountRequest = {
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
      };
      if (getServiceDataSource("resource") === "api") {
        await createServerAccountConnection({
          account: accountRequest,
          serverCode,
          workspaceCode,
        });
      } else {
        createMockManagedResource({
          allowInheritance: false,
          attributes: {
            authenticationType: accountRequest.authenticationType,
            serverCode,
            serverWorkspaceCode: workspaceCode,
            username: accountRequest.username,
          },
          description: accountRequest.description ?? "",
          file: null,
          kind: "SERVER_ACCOUNT",
          name: accountRequest.name,
          resourceCode: accountRequest.accountCode,
          workspaceCode,
        });
      }
      toastManager.add({
        description: pageT("messages.accountCreated"),
        title: pageT("messages.created"),
        type: "success",
      });
      onOpenChange(false);
      onCreated();
    } catch (cause) {
      toastManager.add({
        description: cause instanceof Error ? cause.message : pageT("messages.createFailed"),
        title: pageT("messages.createFailed"),
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
            <DialogTitle>{pageT("serverAccount.createTitle")}</DialogTitle>
          </DialogHeader>
          <div className="grid gap-4 py-5 sm:grid-cols-2">
            <div className="sm:col-span-2">
              <ConnectionSelectField id="account-server" label={pageT("fields.server")} onValueChange={setServerCode} value={serverCode}>
                {servers.map((server) => (
                  <ConnectionSelectItem key={server.resourceCode} value={server.resourceCode}>
                    {server.name} · {server.resourceCode}
                  </ConnectionSelectItem>
                ))}
              </ConnectionSelectField>
            </div>
            <ServerAccountFields idPrefix="server-account" onChange={setAccount} value={account} />
          </div>
          <DialogFooter>
            <Button disabled={submitting} type="button" variant="outline" onClick={() => onOpenChange(false)}>{pageT("actions.cancel")}</Button>
            <Button disabled={submitting || !serverCode} type="submit">
              {submitting ? <Spinner /> : null}
              {submitting ? pageT("actions.creating") : pageT("actions.create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
