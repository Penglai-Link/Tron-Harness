import {
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
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
  Link,
  useLocation,
  useNavigate,
  useSearchParams,
} from "react-router";

import { Button } from "@/components/ui/button";
import {
  createResource,
  deleteResource,
  downloadResourceBinary,
  updateResource,
  uploadResourceBinary,
} from "@/domain/resource/api";
import { useResourceDetail, useSettingsResourceScope } from "@/domain/resource/hooks";
import { createMockQuickResource } from "@/domain/resource/mock";
import type { ResourceKind, ResourceRecord } from "@/domain/resource/model";
import {
  useQuickResourceUploadHandoffStore,
  type QuickResourceUploadHandoff,
} from "@/domain/resource/model";
import {
  ConfirmDialog,
  TextArea,
  TextInput,
  ToggleSwitch,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Badge } from "@/shared/ui/Badge";

const fileKinds: ResourceKind[] = ["FILE", "IMAGE", "PDF"];

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
  const [stagedFile, setStagedFile] = useState<File | null>(null);
  const [record, setRecord] = useState<ResourceRecord | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const initializedFromRef = useRef<string | null>(null);

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
    if (loaded && initializedFromRef.current !== loaded.resourceCode) {
      initializedFromRef.current = loaded.resourceCode;
      setRecord(loaded);
      setAllowInheritance(loaded.allowInheritance);
      setDescription(loaded.description);
      setKind(loaded.kind);
      setName(loaded.name);
      setResourceCode(loaded.resourceCode);
    }
  }, [detailQuery.data, isNew, quickUploadId, scopeReady, toast, workspaceCode]);

  const isFileKind = fileKinds.includes(kind);
  const loadedResource = isNew ? null : record;
  const saveDisabled = saving || !name.trim() || (isNew && (!resourceCode.trim() || !stagedFile));

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
      if (isApi) {
        await updateResource({
          allowInheritance,
          description,
          name: name.trim(),
          resourceCode: loadedResource.resourceCode,
          workspaceCode,
        });
        setRecord({
          ...loadedResource,
          allowInheritance,
          description,
          name: name.trim(),
        });
      }
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
    if (!file || !loadedResource) return;
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
        await deleteResource({
          resourceCode: loadedResource.resourceCode,
          workspaceCode,
        });
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
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("emptyWorkspace", "Select a workspace to manage resources.")}</p>;
  }

  if (!isNew && requestedResourceCode && !detailQuery.loading && !loadedResource) {
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("notFound", "Resource not found.")}</p>;
  }

  const readOnly = !isNew && !editing;

  return (
    <div className="min-w-0">
      <nav aria-label={editorT("breadcrumb.aria", "Breadcrumb")} className="mb-3 flex flex-wrap items-center gap-1 text-[12px] text-[var(--color-on-surface-variant)]">
        <Link className="hover:text-[var(--color-on-surface)]" to={basePath}>
          {editorT("breadcrumb.resources", "Resources")}
        </Link>
        {!isNew ? <><span aria-hidden="true">/</span><span className="font-mono">{resourceCode}</span></> : null}
      </nav>

      <header className="mb-4 border-b border-[var(--color-outline-variant)] pb-4">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0 flex-1">
            <div className="min-w-0 max-w-3xl">
              <TextInput
                aria-label={editorT("fields.name", "Resource name")}
                className="h-11 text-xl font-semibold"
                disabled={readOnly || saving}
                placeholder={editorT("fields.namePlaceholder", "Resource name")}
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </div>

            <div className="mt-2 max-w-3xl">
              <TextArea
                aria-label={editorT("fields.description", "Description")}
                className="min-h-16"
                disabled={readOnly || saving}
                placeholder={editorT("fields.descriptionPlaceholder", "Describe when agents and applications should use this resource")}
                value={description}
                onChange={(event) => setDescription(event.target.value)}
              />
            </div>

            <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-[12px] text-[var(--color-on-surface-variant)]">
              {isNew ? (
                <TextInput
                  aria-label={editorT("fields.code", "Resource code")}
                  className="h-8 w-56 font-mono text-[12px]"
                  disabled={saving}
                  placeholder="resource-code"
                  value={resourceCode}
                  onChange={(event) => setResourceCode(event.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, ""))}
                />
              ) : <span className="font-mono font-medium text-[var(--color-on-surface)]">{resourceCode}</span>}
              <Badge tone="outline">{t(`pages:resources.kind.${kind.toLowerCase()}`, { defaultValue: kind })}</Badge>
              <ToggleSwitch
                checked={allowInheritance}
                disabled={readOnly || saving}
                label={editorT("status.inheritance", "Allow inheritance")}
                onCheckedChange={setAllowInheritance}
              />
            </div>
          </div>

          <div className="flex shrink-0 flex-wrap items-center gap-2">
            {isNew ? (
              <>
                <Button disabled={saving} variant="outline" onClick={() => navigate(basePath)}>
                  {editorT("actions.cancel", "Cancel")}
                </Button>
                <Button disabled={saveDisabled} onClick={() => { void save(); }}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.create", "Create")}
                </Button>
              </>
            ) : editing ? (
              <>
                <Button
                  disabled={saving}
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
                <Button disabled={saving || !name.trim()} onClick={() => { void save(); }}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.saveActive", "Save")}
                </Button>
              </>
            ) : (
              <>
                <Button onClick={() => setEditing(true)}>
                  <Pencil aria-hidden="true" size={15} />
                  {editorT("actions.edit", "Edit")}
                </Button>
                <Button variant="outline" onClick={() => setDeleteOpen(true)}>
                  <Trash2 aria-hidden="true" size={15} />
                  {editorT("actions.delete", "Delete resource")}
                </Button>
              </>
            )}
          </div>
        </div>
      </header>

      <div className="grid min-w-0 items-start gap-4 xl:grid-cols-[minmax(0,1.55fr)_minmax(320px,0.75fr)]">
        <section className="min-w-0 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-4 sm:p-5">
          <h2 className="mb-3 text-sm font-semibold">{editorT("tabs.data", "Resource information")}</h2>
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
              <Button disabled={saving} type="button" variant="outline" onClick={() => fileInputRef.current?.click()}>
                <FileUp aria-hidden="true" size={15} />
                {editorT("file.chooseFile", "Choose a file")}
              </Button>
              {stagedFile ? (
                <p className="text-[12px] text-[var(--color-on-surface-variant)]">
                  {stagedFile.name} · {formatByteSize(stagedFile.size)}
                </p>
              ) : null}
            </div>
          ) : loadedResource && isFileKind ? (
            <div className="space-y-3">
              {loadedResource.file ? (
                <div className="flex flex-wrap items-center gap-3 rounded-lg border border-[var(--color-outline-variant)] px-3 py-2.5">
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
              <Button disabled={saving} type="button" variant="outline" onClick={() => fileInputRef.current?.click()}>
                <FileUp aria-hidden="true" size={15} />
                {editorT("file.replace", "Replace file")}
              </Button>
            </div>
          ) : loadedResource && loadedResource.attributes ? (
            <pre className="overflow-x-auto rounded-lg bg-[var(--color-surface-container-low)] p-3 font-mono text-[12px] leading-5">
              {JSON.stringify(loadedResource.attributes, null, 2)}
            </pre>
          ) : (
            <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("emptyAttributes", "This resource has no attributes.")}</p>
          )}
        </section>
      </div>

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
    </div>
  );
}
