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
import { createServerConnection } from "@/domain/resource/api";
import { createMockManagedResource } from "@/domain/resource/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  ConnectionTextareaField,
  ConnectionTextField,
} from "./ConnectionFormFields";
import { generateConnectionResourceCode } from "./connectionResourceCode";

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
  const [submitting, setSubmitting] = useState(false);
  const pageT = (key: string) => t(`resourceConnections.${key}`);

  useEffect(() => {
    if (!open) return;
    setServerCode(generateConnectionResourceCode("SERVER"));
  }, [open]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const request = {
        allowInheritance: false,
        description: description.trim() || null,
        host: host.trim(),
        name: name.trim(),
        port: Number(port),
        serverCode: serverCode.trim(),
        workspaceCode,
      };
      const isApi = getServiceDataSource("resource") === "api";
      if (isApi) await createServerConnection(request);
      if (!isApi) {
        createMockManagedResource({
          allowInheritance: false,
          attributes: { host: request.host, port: request.port, protocol: "Ssh" },
          description: request.description ?? "",
          file: null,
          kind: "SERVER_CONNECTION",
          name: request.name,
          resourceCode: request.serverCode,
          workspaceCode,
        });
      }
      toastManager.add({
        description: pageT("messages.serverCreated"),
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
          </div>
          <DialogFooter>
            <Button disabled={submitting} type="button" variant="outline" onClick={() => onOpenChange(false)}>{pageT("actions.cancel")}</Button>
            <Button disabled={submitting} type="submit">

              {submitting ? pageT("actions.creating") : pageT("actions.create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
