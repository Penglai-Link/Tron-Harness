import {
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Download,
  FileUp,
  Pencil,
  Save,
  Trash2,
} from "lucide-react";
import {
  useLocation,
  useNavigate,
  useSearchParams,
} from "react-router";

import { Button } from "@/components/ui/button";
import {
  createResource,
  deleteResource,
  downloadResourceBinary,
  updateDataSourceConnection,
  updateResource,
  uploadResourceBinary,
} from "@/domain/resource/api";
import { useResourceDetail, useSettingsResourceScope } from "@/domain/resource/hooks";
import {
  createMockQuickResource,
  deleteMockResource,
  replaceMockResourceFile,
  updateMockResource,
} from "@/domain/resource/mock";
import type { ResourceKind, ResourceRecord } from "@/domain/resource/model";
import {
  useQuickResourceUploadHandoffStore,
  type QuickResourceUploadHandoff,
} from "@/domain/resource/model";
import { Field, TextArea, TextInput, ToggleSwitch } from "@/components/forms";
import {
  PageHeader,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useToast } from "@/components/ui/toast";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Badge } from "@/components/ui/badge";

const fileKinds: ResourceKind[] = ["FILE", "IMAGE", "PDF"];
const dataSourceKinds: ResourceKind[] = [
  "MYSQL_CONNECTION",
  "POSTGRESQL_CONNECTION",
  "REDIS_CONNECTION",
  "KAFKA_CONNECTION",
];

function formatByteSize(byteSize: number) {
  if (byteSize < 1024) return `${byteSize} B`;
  if (byteSize < 1024 * 1024) return `${(byteSize / 1024).toFixed(1)} KB`;
  return `${(byteSize / (1024 * 1024)).toFixed(1)} MB`;
}

function downloadFile(blob: Blob, fallbackName: string, contentDisposition: string | null) {
  const encodedFilename = contentDisposition?.match(/filename\*=UTF-8''([^;]+)/i)?.[1];
  const quotedFilename = contentDisposition?.match(/filename="([^"]+)"/i)?.[1]
    ?? contentDisposition?.match(/filename=([^;\s]+)/i)?.[1];
  let fileName = fallbackName;
  if (encodedFilename) {
    try {
      fileName = decodeURIComponent(encodedFilename);
    } catch {
      fileName = fallbackName;
    }
  } else if (quotedFilename) {
    fileName = quotedFilename;
  }

  const objectUrl = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.download = fileName;
  anchor.href = objectUrl;
  anchor.style.display = "none";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
}

function requestErrorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Resource request failed";
}

function ResourceDetailItem({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="min-w-0 border-b border-[var(--color-outline-variant)] pb-3 last:border-b-0 sm:last:border-b">
      <dt className="text-xs font-medium text-[var(--color-on-surface-variant)]">{label}</dt>
      <dd className="mt-1 min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
        {value || <span className="text-[var(--color-on-surface-variant)]">—</span>}
      </dd>
    </div>
  );
}

function displayResourceAttribute(attributes: Record<string, unknown> | null | undefined, key: string) {
  const value = attributes?.[key];
  return typeof value === "string" || typeof value === "number" ? String(value) : "—";
}

export function ResourceEditorPage() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const consoleWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const settingsScope = useSettingsResourceScope();
  const activeWorkspace = settingsScope.isSettings
    ? (settingsScope.workspaceCode ? { workspaceCode: settingsScope.workspaceCode, name: settingsScope.workspaceName } : undefined)
    : consoleWorkspace;
  const resourceManagementBasePath = location.pathname.match(/^\/resource-management\/[^/]+/)?.[0];
  const basePath = settingsScope.isSettings ? "/settings/resources" : resourceManagementBasePath ?? "/resources";
  const workspaceCode = activeWorkspace?.workspaceCode ?? "";
  const workspaceName = activeWorkspace?.name ?? "";
  const scopeReady = Boolean(activeWorkspace);
  const isApi = getServiceDataSource("resource") === "api";
  const isNew = location.pathname.endsWith("/new");
  const requestedResourceCode = searchParams.get("resourceCode") ?? "";
  const quickUploadId = searchParams.get("quickUpload");
  const quickUploadHandoff = useRef<QuickResourceUploadHandoff | null | undefined>(undefined);
  const quickUploadWarningShown = useRef(false);
  if (quickUploadHandoff.current === undefined) {
    quickUploadHandoff.current = quickUploadId
      ? useQuickResourceUploadHandoffStore.getState().peek(quickUploadId)
      : null;
  }
  const editorT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:resourceEditor.${key}`, { defaultValue, ...options })
  );
  const listLabel = resourceManagementBasePath === "/resource-management/files"
    ? t("pages:resourceConnections.file.title", { defaultValue: "File Management" })
    : resourceManagementBasePath === "/resource-management/data-sources"
    ? t("pages:resourceConnections.dataSource.title", { defaultValue: "Data Source Management" })
    : resourceManagementBasePath === "/resource-management/servers"
    ? t("pages:resourceConnections.server.title", { defaultValue: "Server Management" })
    : resourceManagementBasePath === "/resource-management/server-accounts"
    ? t("pages:resourceConnections.serverAccount.title", { defaultValue: "Server Account Management" })
    : settingsScope.isSettings
    ? t("pages:resources.title.platform", { defaultValue: "Platform Resources" })
    : editorT("breadcrumb.resources", "Resources");
  const newResourceTitle = editorT("titles.create", "New Resource");
  const resourceDetailTitle = editorT("titles.detail", "Resource Details");

  const detailQuery = useResourceDetail({
    resourceCode: isNew ? "" : requestedResourceCode,
    workspaceCode: workspaceCode || null,
    workspaceName,
  });

  const [editing, setEditing] = useState(isNew);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [allowInheritance, setAllowInheritance] = useState(false);
  const [resourceCode, setResourceCode] = useState("");
  const [kind, setKind] = useState<ResourceKind>("FILE");
  const [attributesText, setAttributesText] = useState("{}");
  const [stagedFile, setStagedFile] = useState<File | null>(null);
  const [record, setRecord] = useState<ResourceRecord | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const initializedFromRef = useRef<string | null>(null);

  useEffect(() => {
    if (isNew) return;
    const key = `${workspaceCode}:${requestedResourceCode}`;
    if (initializedFromRef.current === key) return;
    setEditing(false);
    setRecord(null);
    setAllowInheritance(false);
    setAttributesText("{}");
    setDescription("");
    setName("");
    setResourceCode(requestedResourceCode);
  }, [isNew, requestedResourceCode, workspaceCode]);

  useEffect(() => {
    if (!scopeReady) return;
    if (isNew) {
      const handoff = quickUploadHandoff.current;
      if (handoff && handoff.workspaceCode === workspaceCode) {
        if (initializedFromRef.current !== quickUploadId) {
          initializedFromRef.current = quickUploadId;
          setAllowInheritance(handoff.allowInheritance);
          setDescription(handoff.description);
          setKind(handoff.kind);
          setName(handoff.name);
          setResourceCode(handoff.resourceCode);
          setStagedFile(handoff.file);
          if (quickUploadId) useQuickResourceUploadHandoffStore.getState().clear(quickUploadId);
          if (!quickUploadWarningShown.current) {
            quickUploadWarningShown.current = true;
            toast.info(editorT("messages.quickUploadRestored", "File and detected details were added. Complete the remaining fields before creating the resource."));
          }
        }
        return;
      }
      if (quickUploadId && !quickUploadWarningShown.current) {
        quickUploadWarningShown.current = true;
        toast.warning(editorT(
          handoff
            ? "messages.quickUploadScopeMismatch"
            : "messages.quickUploadFileMissing",
          handoff
            ? "The quick-upload file belongs to a different workspace. Choose it again from this Resource list."
            : "The selected quick-upload file is no longer available. Choose it again in Files to continue.",
        ));
      }
      return;
    }

    const loaded = detailQuery.data;
    const loadedKey = `${workspaceCode}:${loaded?.resourceCode ?? ""}`;
    if (loaded && loaded.workspaceCode === workspaceCode && initializedFromRef.current !== loadedKey) {
      initializedFromRef.current = loadedKey;
      setRecord(loaded);
      setAllowInheritance(loaded.allowInheritance);
      setAttributesText(JSON.stringify(loaded.attributes ?? {}, null, 2));
      setDescription(loaded.description);
      setKind(loaded.kind);
      setName(loaded.name);
      setResourceCode(loaded.resourceCode);
    }
  }, [detailQuery.data, isNew, quickUploadId, scopeReady, toast, workspaceCode]);

  const isFileKind = fileKinds.includes(kind);
  const isDataSource = dataSourceKinds.includes(kind);
  const isServerKind = kind === "SERVER_CONNECTION";
  const loadedResource = isNew ? null : record;
  const inherited = loadedResource?.sourceKind === "INHERITED";
  const saveDisabled = saving || inherited || !name.trim() || (isNew && (!resourceCode.trim() || !stagedFile));

  async function save() {
    if (saveDisabled) return;
    setSaving(true);
    try {
      if (isNew) {
        if (!stagedFile) return;
        if (isApi) {
          const result = await createResource({
            allowInheritance,
            description,
            file: stagedFile,
            kind: kind as "FILE" | "IMAGE" | "PDF",
            name: name.trim(),
            resourceCode: resourceCode.trim(),
            workspaceCode,
          });
          toast.success(editorT("messages.created", "{{name}} created", { name: name.trim() }));
          navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(result.resourceCode)}`, { replace: true });
          return;
        }
        const created = createMockQuickResource({
          allowInheritance,
          description,
          file: stagedFile,
          kind: kind as "FILE" | "IMAGE" | "PDF",
          name: name.trim(),
          pageCount: null,
          resourceCode: resourceCode.trim(),
          workspaceCode,
        });
        toast.success(editorT("messages.created", "{{name}} created", { name: created.name }));
        navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(created.resourceCode)}`, { replace: true });
        return;
      }

      if (!loadedResource) return;
      let attributes = loadedResource.attributes;
      if (isDataSource) {
        try {
          const parsed: unknown = JSON.parse(attributesText);
          if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
            throw new Error(editorT("messages.invalidAttributes", "Connection attributes must be a JSON object."));
          }
          attributes = parsed as Record<string, unknown>;
        } catch (cause) {
          throw cause instanceof Error
            ? cause
            : new Error(editorT("messages.invalidAttributes", "Connection attributes must be a JSON object."));
        }
      }
      if (isApi) {
        if (isDataSource) {
          await updateDataSourceConnection({
            allowInheritance,
            attributes: attributes ?? {},
            description,
            name: name.trim(),
            resourceCode: loadedResource.resourceCode,
            workspaceCode,
          });
        } else {
          await updateResource({
            allowInheritance,
            description,
            name: name.trim(),
            resourceCode: loadedResource.resourceCode,
            workspaceCode,
          });
        }
      }
      const updated = isApi
        ? {
            ...loadedResource,
            allowInheritance,
            attributes,
            description,
            name: name.trim(),
          }
        : updateMockResource(workspaceCode, loadedResource.resourceCode, {
            allowInheritance,
            attributes,
            description,
            name: name.trim(),
          });
      if (!updated) {
        throw new Error(editorT("messages.saveFailed", "Resource could not be saved."));
      }
      setRecord(updated);
      setEditing(false);
      toast.success(editorT("messages.saved", "Resource saved"));
    } catch (cause) {
      toast.error(requestErrorMessage(cause));
    } finally {
      setSaving(false);
    }
  }

  async function replaceFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file || !loadedResource || inherited) return;
    if (isApi) {
      try {
        const result = await uploadResourceBinary({
          file,
          resourceCode: loadedResource.resourceCode,
          workspaceCode,
        });
        setRecord({
          ...loadedResource,
          file: {
            byteSize: result.byteSize,
            fileExtension: result.fileName.includes(".") ? result.fileName.slice(result.fileName.lastIndexOf(".") + 1).toLowerCase() : "",
            fileName: result.fileName,
          },
        });
        toast.success(editorT("messages.replaced", "{{name}} replaced with {{file}}", { file: result.fileName, name: loadedResource.name }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      }
      return;
    }
    const updated = replaceMockResourceFile(workspaceCode, loadedResource.resourceCode, file);
    if (!updated) {
      toast.error(editorT("messages.replaceFailed", "Resource could not be replaced."));
      return;
    }
    setRecord(updated);
    toast.success(editorT("messages.replaced", "{{name}} replaced with {{file}}", { file: file.name, name: loadedResource.name }));
  }

  async function downloadCurrentFile() {
    if (!loadedResource?.file) return;
    if (isApi) {
      try {
        const response = await downloadResourceBinary({
          resourceCode: loadedResource.resourceCode,
          workspaceCode,
        });
        downloadFile(response.blob, loadedResource.file.fileName, response.contentDisposition);
        toast.info(editorT("messages.downloaded", "Download started for {{name}}", { name: loadedResource.file.fileName }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      }
      return;
    }
    toast.info(editorT("messages.downloaded", "Download started for {{name}}", { name: loadedResource.file.fileName }));
  }

  async function deleteCurrentResource() {
    if (!loadedResource) return;
    setDeleting(true);
    try {
      if (isApi) {
        const result = await deleteResource({
          resourceCode: loadedResource.resourceCode,
          workspaceCode,
        });
        if (!result.deleted) throw new Error(editorT("messages.deleteFailed", "Resource was not deleted."));
      } else if (!deleteMockResource(workspaceCode, loadedResource.resourceCode)) {
        throw new Error(editorT("messages.deleteFailed", "Resource was not deleted."));
      }
      setDeleteOpen(false);
      toast.success(editorT("messages.deleted", "{{name}} deleted", { name: loadedResource.name }));
      navigate(basePath);
    } catch (cause) {
      toast.error(requestErrorMessage(cause));
    } finally {
      setDeleting(false);
    }
  }

  if (!scopeReady) {
    return (
      <>
        <PageHeader title={isNew ? newResourceTitle : listLabel} />
        <SurfaceCard className="max-w-5xl text-sm text-[var(--color-on-surface-variant)]">
          {editorT("emptyWorkspace", "Select a workspace to manage resources.")}
        </SurfaceCard>
      </>
    );
  }

  if (!isNew && requestedResourceCode && !detailQuery.loading && !loadedResource) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ href: basePath, label: listLabel }, { label: resourceDetailTitle }]}
          title={resourceDetailTitle}
        />
        <SurfaceCard className="max-w-5xl text-sm text-[var(--color-on-surface-variant)]">
          {editorT("notFound", "Resource not found.")}
        </SurfaceCard>
      </>
    );
  }

  const readOnly = inherited || (!isNew && !editing);

  if (!isNew && isServerKind && loadedResource) {
    return (
      <>
        <PageHeader
          breadcrumbs={[
            { href: basePath, label: listLabel },
            { label: loadedResource.resourceCode },
          ]}
          title={loadedResource.name}
          actions={(
            <>
              {editing ? (
                <>
                  <Button
                    disabled={saving}
                    size="lg"
                    variant="outline"
                    onClick={() => {
                      setEditing(false);
                      setAllowInheritance(loadedResource.allowInheritance);
                      setDescription(loadedResource.description);
                      setName(loadedResource.name);
                    }}
                  >
                    {editorT("actions.cancel", "Cancel")}
                  </Button>
                  <Button disabled={saving || !name.trim()} size="lg" onClick={() => { void save(); }}>
                    <Save aria-hidden="true" size={15} />
                    {editorT("actions.saveActive", "Save")}
                  </Button>
                </>
              ) : inherited ? null : (
                <>
                  <Button size="lg" variant="outline" onClick={() => setEditing(true)}>
                    <Pencil aria-hidden="true" size={15} />
                    {editorT("actions.edit", "Edit")}
                  </Button>
                  <Button size="lg" variant="outline" onClick={() => setDeleteOpen(true)}>
                    <Trash2 aria-hidden="true" size={15} />
                    {editorT("actions.delete", "Delete resource")}
                  </Button>
                </>
              )}
            </>
          )}
        />

        <SurfaceCard className="max-w-5xl space-y-5">
          <section className="border-b border-[var(--color-outline-variant)] pb-5">
            <h2 className="mb-5 text-base font-semibold text-[var(--color-on-surface)]">
              {editorT("sections.basic", "Basic information")}
            </h2>
            <dl className="grid gap-x-8 gap-y-4 sm:grid-cols-2">
              <ResourceDetailItem
                label={editorT("fields.code", "Resource code")}
                value={<span className="font-mono">{loadedResource.resourceCode}</span>}
              />
              <ResourceDetailItem
                label={t("pages:resources.kind.server_connection", "Server connection")}
                value={t("pages:resources.source." + loadedResource.sourceKind.toLowerCase(), loadedResource.sourceKind)}
              />
              <div className="min-w-0 border-b border-[var(--color-outline-variant)] pb-3 sm:col-span-2">
                <dt className="text-xs font-medium text-[var(--color-on-surface-variant)]">
                  {editorT("fields.name", "Resource name")}
                </dt>
                <dd className="mt-1">
                  {editing ? (
                    <TextInput
                      aria-label={editorT("fields.name", "Resource name")}
                      disabled={saving}
                      value={name}
                      onChange={(event) => setName(event.target.value)}
                    />
                  ) : (
                    <span className="text-sm leading-6 text-[var(--color-on-surface)]">{loadedResource.name}</span>
                  )}
                </dd>
              </div>
              <div className="min-w-0 border-b border-[var(--color-outline-variant)] pb-3 sm:col-span-2">
                <dt className="text-xs font-medium text-[var(--color-on-surface-variant)]">
                  {editorT("fields.description", "Description")}
                </dt>
                <dd className="mt-1">
                  {editing ? (
                    <TextArea
                      aria-label={editorT("fields.description", "Description")}
                      className="min-h-20"
                      disabled={saving}
                      value={description}
                      onChange={(event) => setDescription(event.target.value)}
                    />
                  ) : (
                    <span className="whitespace-pre-wrap text-sm leading-6 text-[var(--color-on-surface)]">
                      {loadedResource.description || "—"}
                    </span>
                  )}
                </dd>
              </div>
              <div className="sm:col-span-2">
                <ToggleSwitch
                  checked={allowInheritance}
                  disabled={readOnly || saving}
                  label={editorT("status.inheritance", "Allow inheritance")}
                  onCheckedChange={setAllowInheritance}
                />
              </div>
            </dl>
          </section>

          <section>
            <h2 className="mb-5 text-base font-semibold text-[var(--color-on-surface)]">
              {editorT("sections.connection", "Connection")}
            </h2>
            <dl className="grid gap-x-8 gap-y-4 sm:grid-cols-3">
              <ResourceDetailItem
                label={t("pages:resourceConnections.fields.host", "Host")}
                value={<span className="font-mono">{displayResourceAttribute(loadedResource.attributes, "host")}</span>}
              />
              <ResourceDetailItem
                label={t("pages:resourceConnections.fields.port", "Port")}
                value={<span className="font-mono">{displayResourceAttribute(loadedResource.attributes, "port")}</span>}
              />
              <ResourceDetailItem
                label={t("pages:resourceConnections.columns.protocol", "Protocol")}
                value={displayResourceAttribute(loadedResource.attributes, "protocol")}
              />
            </dl>
          </section>
        </SurfaceCard>

        <ConfirmDialog
          cancelText={editorT("actions.cancel", "Cancel")}
          confirmText={editorT("actions.delete", "Delete resource")}
          description={editorT("messages.deleteDescription", "Delete this resource and its files. This cannot be undone.")}
          open={deleteOpen}
          pending={deleting}
          title={editorT("messages.deleteTitle", "Delete {{name}}?", { name: name })}
          onCancel={() => setDeleteOpen(false)}
          onConfirm={() => { void deleteCurrentResource(); }}
        />
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { href: basePath, label: listLabel },
          ...(isNew ? [] : [{ label: resourceCode || requestedResourceCode }]),
        ]}
        title={isNew ? newResourceTitle : loadedResource?.name ?? resourceDetailTitle}
        actions={(
          <>
            {isNew ? (
              <>
                <Button
                  disabled={saving}
                  size="lg"
                  variant="outline"
                  onClick={() => navigate(basePath)}
                >
                  {editorT("actions.cancel", "Cancel")}
                </Button>
                <Button disabled={saveDisabled} size="lg" onClick={() => { void save(); }}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.create", "Create")}
                </Button>
              </>
            ) : editing ? (
              <>
                <Button
                  disabled={saving}
                  size="lg"
                  variant="outline"
                  onClick={() => {
                    if (!loadedResource) return;
                    setEditing(false);
                    setAllowInheritance(loadedResource.allowInheritance);
                    setDescription(loadedResource.description);
                    setName(loadedResource.name);
                  }}
                >
                  {editorT("actions.cancel", "Cancel")}
                </Button>
                <Button disabled={saving || !name.trim()} size="lg" onClick={() => { void save(); }}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.saveActive", "Save")}
                </Button>
              </>
            ) : inherited ? null : (
              <>
                <Button size="lg" variant="outline" onClick={() => setEditing(true)}>
                  <Pencil aria-hidden="true" size={15} />
                  {editorT("actions.edit", "Edit")}
                </Button>
                <Button size="lg" variant="outline" onClick={() => setDeleteOpen(true)}>
                  <Trash2 aria-hidden="true" size={15} />
                  {editorT("actions.delete", "Delete resource")}
                </Button>
              </>
            )}
          </>
        )}
      />

      <SurfaceCard className="max-w-5xl space-y-5">
        <div className="grid gap-4 md:grid-cols-2">
          <Field label={editorT("fields.name", "Resource name")}>
            <TextInput
              aria-label={editorT("fields.name", "Resource name")}
              disabled={readOnly || saving}
              placeholder={editorT("fields.namePlaceholder", "Resource name")}
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </Field>
          <Field label={editorT("fields.code", "Resource code")}>
            {isNew ? (
              <TextInput
                aria-label={editorT("fields.code", "Resource code")}
                className="font-mono"
                disabled={saving}
                placeholder={editorT("fields.codePlaceholder", "resource-code")}
                value={resourceCode}
                onChange={(event) => setResourceCode(event.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, ""))}
              />
            ) : (
              <TextInput
                aria-label={editorT("fields.code", "Resource code")}
                className="font-mono"
                readOnly
                value={resourceCode || requestedResourceCode}
              />
            )}
          </Field>
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          <Field label={editorT("fields.type", "Resource type")}>
            <div className="flex h-10 items-center">
              <Badge variant="outline">
                {t(`pages:resources.kind.${kind.toLowerCase()}`, {
                  defaultValue: kind,
                })}
              </Badge>
            </div>
          </Field>
          <div className="flex min-h-10 items-end">
            <ToggleSwitch
              checked={allowInheritance}
              disabled={readOnly || saving}
              label={editorT("status.inheritance", "Allow inheritance")}
              onCheckedChange={setAllowInheritance}
            />
          </div>
        </div>

        <Field label={editorT("fields.description", "Description")}>
          <TextArea
            aria-label={editorT("fields.description", "Description")}
            className="min-h-24 resize-y"
            disabled={readOnly || saving}
            placeholder={editorT("fields.descriptionPlaceholder", "Describe when agents and applications should use this resource")}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </Field>

        <section className="border-t border-[var(--color-outline-variant)] pt-5">
          <h2 className="mb-4 text-base font-semibold text-[var(--color-on-surface)]">
            {editorT("tabs.data", "Resource information")}
          </h2>
          {isFileKind && isNew ? (
            <div className="space-y-3">
              <input
                aria-label={editorT("file.chooseFile", "Choose a file")}
                className="sr-only"
                disabled={saving}
                id="resource-editor-file-input"
                ref={fileInputRef}
                type="file"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  event.target.value = "";
                  if (file) setStagedFile(file);
                }}
              />
              <Button
                disabled={saving}
                type="button"
                variant="outline"
                onClick={() => fileInputRef.current?.click()}
              >
                <FileUp aria-hidden="true" size={15} />
                {editorT("file.chooseFile", "Choose a file")}
              </Button>
              {stagedFile ? (
                <div className="flex min-w-0 items-center justify-between gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-3 py-2.5">
                  <span className="min-w-0 truncate text-sm font-medium">{stagedFile.name}</span>
                  <span className="shrink-0 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                    {formatByteSize(stagedFile.size)}
                  </span>
                </div>
              ) : null}
            </div>
          ) : loadedResource && isFileKind ? (
            <div className="space-y-3">
              {loadedResource.file ? (
                <div className="flex flex-wrap items-center gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-3 py-2.5">
                  <span className="min-w-0 flex-1 truncate text-sm font-medium">{loadedResource.file.fileName}</span>
                  <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">{formatByteSize(loadedResource.file.byteSize)}</span>
                  <Button size="sm" type="button" variant="outline" onClick={() => { void downloadCurrentFile(); }}>
                    <Download aria-hidden="true" size={14} />
                    {editorT("file.download", "Download")}
                  </Button>
                </div>
              ) : (
                <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("file.empty", "No file uploaded yet.")}</p>
              )}
              <input
                aria-label={editorT("file.replace", "Replace file")}
                className="sr-only"
                disabled={saving}
                id="resource-editor-replace-input"
                ref={fileInputRef}
                type="file"
                onChange={(event) => { void replaceFile(event); }}
              />
              {!inherited ? (
                <Button disabled={saving} type="button" variant="outline" onClick={() => fileInputRef.current?.click()}>
                  <FileUp aria-hidden="true" size={15} />
                  {editorT("file.replace", "Replace file")}
                </Button>
              ) : null}
            </div>
          ) : loadedResource && isDataSource && editing ? (
            <div className="space-y-2">
              <TextArea
                aria-label={editorT("fields.connectionAttributes", "Connection attributes")}
                className="min-h-72 font-mono text-[12px]"
                disabled={saving}
                value={attributesText}
                onChange={(event) => setAttributesText(event.target.value)}
              />
              <p className="text-xs text-[var(--color-on-surface-variant)]">
                {editorT("hints.directSecretRedacted", "Direct secret values are redacted. Leave their null value unchanged to retain the existing secret, or enter a replacement value.")}
              </p>
            </div>
          ) : loadedResource && loadedResource.attributes ? (
            <pre className="overflow-x-auto rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-3 py-3 font-mono text-[12px] leading-5">
              {JSON.stringify(loadedResource.attributes, null, 2)}
            </pre>
          ) : (
            <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("emptyAttributes", "This resource has no attributes.")}</p>
          )}
        </section>
      </SurfaceCard>

      <ConfirmDialog
        cancelText={editorT("actions.cancel", "Cancel")}
        confirmText={editorT("actions.delete", "Delete resource")}
        description={editorT("messages.deleteDescription", "Delete this resource and its files. This cannot be undone.")}
        open={deleteOpen}
        pending={deleting}
        title={editorT("messages.deleteTitle", "Delete {{name}}?", { name: name })}
        onCancel={() => setDeleteOpen(false)}
        onConfirm={() => { void deleteCurrentResource(); }}
      />
    </>
  );
}
