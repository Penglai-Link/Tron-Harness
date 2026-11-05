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

import { toastManager } from "@/components/ui/toast";
import { createServerAccountConnection, updateServerAccountConnection } from "@/domain/resource/api";
import type { ResourceRecord } from "@/domain/resource/model";
import { createMockManagedResource, updateMockResource } from "@/domain/resource/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

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
  record?: ResourceRecord | null;
  workspaceCode: string;
}

export function ServerAccountCreateDialog({
  onCreated,
  onOpenChange,
  open,
  record,
  workspaceCode,
}: ServerAccountCreateDialogProps) {
  const { t } = useTranslation("pages");
  const [account, setAccount] = useState<ServerAccountFormValue>({ ...emptyServerAccount });
  const [submitting, setSubmitting] = useState(false);
  const pageT = (key: string) => t(`resourceConnections.${key}`);

  useEffect(() => {
    setAccount(open ? {
      ...emptyServerAccount,
      accountCode: record?.resourceCode ?? generateConnectionResourceCode("SERVER_ACCOUNT"),
      name: record?.name ?? "",
      description: record?.description ?? "",
      username: typeof record?.attributes?.username === "string" ? record.attributes.username : "",
      authenticationType: record?.attributes?.authenticationType === "PrivateKey" ? "PrivateKey" : "Password",
    } : { ...emptyServerAccount });
  }, [open, record, workspaceCode]);

  const credentialsRequired = !record || record.attributes?.credentialConfigured !== true
    || account.authenticationType !== record.attributes?.authenticationType;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const accountRequest = {
        accountCode: account.accountCode.trim(),
        allowInheritance: record?.allowInheritance ?? false,
        authenticationType: account.authenticationType,
        description: account.description.trim() || null,
        name: account.name.trim(),
        password: account.authenticationType === "Password" ? account.password || null : null,
        privateKey: account.authenticationType === "PrivateKey" ? account.privateKey || null : null,
        privateKeyPassphrase: account.authenticationType === "PrivateKey" && account.privateKey
          ? account.privateKeyPassphrase || null : null,
        username: account.username.trim(),
      };
      if (getServiceDataSource("resource") === "api") {
        if (record) {
          const { accountCode: _, ...changes } = accountRequest;
          await updateServerAccountConnection(record.resourceCode, { ...changes, workspaceCode });
        } else {
          await createServerAccountConnection({ account: accountRequest, workspaceCode });
        }
      } else {
        // Mock resources keep public metadata only; credentials never enter the mock store.
        const metadata = {
          allowInheritance: accountRequest.allowInheritance,
          attributes: {
            servers: record?.attributes?.servers ?? [],
            credentialConfigured: true,
            authenticationType: accountRequest.authenticationType,
            username: accountRequest.username,
          },
          description: accountRequest.description ?? "",
          name: accountRequest.name,
        };
        if (record) {
          if (!updateMockResource(workspaceCode, record.resourceCode, metadata)) throw new Error(pageT("serverAccount.saveFailed"));
        } else {
          createMockManagedResource({ ...metadata, file: null, kind: "SERVER_ACCOUNT", resourceCode: accountRequest.accountCode, workspaceCode });
        }
      }
      setAccount({ ...emptyServerAccount });
      toastManager.add({
        description: pageT(record ? "serverAccount.saved" : "messages.accountCreated"),
        title: pageT(record ? "serverAccount.saved" : "messages.created"),
        type: "success",
      });
      onOpenChange(false);
      onCreated();
    } catch (cause) {
      toastManager.add({
        description: cause instanceof Error ? cause.message : pageT(record ? "serverAccount.saveFailed" : "messages.createFailed"),
        title: pageT(record ? "serverAccount.saveFailed" : "messages.createFailed"),
        type: "error",
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog onOpenChange={(next) => { if (!submitting) onOpenChange(next); }} open={open}>
      <DialogContent className="max-h-[88vh] overflow-y-auto sm:max-w-2xl">
        <form onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>{pageT(record ? "serverAccount.editTitle" : "serverAccount.createTitle")}</DialogTitle>
          </DialogHeader>
          <fieldset disabled={submitting} className="grid gap-4 py-5 sm:grid-cols-2">
            <ServerAccountFields credentialsRequired={credentialsRequired} idPrefix="server-account" onChange={setAccount} value={account} />
          </fieldset>
          <DialogFooter>
            <Button disabled={submitting} type="button" variant="outline" onClick={() => onOpenChange(false)}>{pageT("actions.cancel")}</Button>
            <Button disabled={submitting || !workspaceCode || !account.name.trim() || !account.username.trim()} type="submit">

              {submitting ? pageT("serverAccount.saving") : pageT(record ? "serverAccount.save" : "actions.create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
