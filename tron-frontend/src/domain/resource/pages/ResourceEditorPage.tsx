import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  MoreHorizontal,
  Pencil,
  Save,
  Send,
} from "lucide-react";
import {
  Link,
  useLocation,
  useNavigate,
  useSearchParams,
} from "react-router";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Button,
  buttonVariants,
} from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import {
  ResourceBinaryEditor,
  ResourceVersionHistory,
  ResourceVersionSource,
} from "@/domain/resource/components";
import type { ResourceVersionHistoryAction } from "@/domain/resource/components";
import {
  useActivePlatformResourceDefinitions,
  useActiveResourceDefinitions,
  usePlatformResourceVersionDetails,
  useResourceVersionDetails,
} from "@/domain/resource/hooks";
import {
  createWorkspaceResourceDraft,
  readResourceVersionsFromUiSession,
  writeResourceVersionsToUiSession,
} from "@/domain/resource/mock";
import type {
  ResourceBinaryMapping,
  ResourceVersionDetail,
} from "@/domain/resource/model";
import {
  Badge,
  SelectInput,
  TextArea,
  TextInput,
  ToggleSwitch,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { cn } from "@/shared/lib/utils";
import {
  createJsonSchemaDraft7FormValue,
  JsonSchemaDraft7Form,
  validateJsonValue,
} from "@/shared/ui/json-schema-draft7";

type EditorSection = "data" | "history";
type InlineField = "description" | "name" | null;

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function cloneVersion(record: ResourceVersionDetail) {
  return cloneJson(record);
}

function validateBinaries(record: ResourceVersionDetail) {
  const errors: string[] = [];
  const { binaries, binarySchema } = record;
  if (binaries.length < binarySchema.minCount) errors.push("binary count is below the minimum");
  if (binaries.length > binarySchema.maxCount) errors.push("binary count exceeds the maximum");
  const names = new Set<string>();
  let namePattern: RegExp | null = null;
  try {
    namePattern = new RegExp(binarySchema.namePattern);
  } catch {
    errors.push("binary name pattern is invalid");
  }
  binaries.forEach(({ binaryName }) => {
    if (!binaryName || binaryName.length > binarySchema.maxNameLength) errors.push(`${binaryName}: invalid binary name length`);
    if (namePattern && !namePattern.test(binaryName)) errors.push(`${binaryName}: binary name does not match the definition`);
    if (names.has(binaryName)) errors.push(`${binaryName}: duplicate binary name`);
    names.add(binaryName);
  });
  return errors;
}

export function ResourceEditorPage() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const isPlatform = location.pathname.startsWith("/settings/");
  const basePath = isPlatform ? "/settings/resources" : "/resources";
  const scopeCode = isPlatform ? "PLATFORM" : activeWorkspace?.workspaceCode ?? "";
  const scopeName = isPlatform ? "Platform" : activeWorkspace?.name ?? "";
  const scopeReady = isPlatform || Boolean(activeWorkspace);
  const isNew = location.pathname.endsWith("/new");
  const requestedResourceCode = searchParams.get("resourceCode") ?? (isPlatform ? "production-mysql" : "storefront-product-gallery");
  const requestedVersion = searchParams.get("resourceVersion") ?? (isPlatform ? "v3-A8n" : "v4-M7q");
  const workspaceSourceVersions = useResourceVersionDetails(
    scopeCode,
    scopeName,
    requestedResourceCode,
  );
  const platformSourceVersions = usePlatformResourceVersionDetails(requestedResourceCode);
  const sourceVersions = isPlatform ? platformSourceVersions : workspaceSourceVersions;
  const workspaceActiveDefinitions = useActiveResourceDefinitions(
    scopeCode,
    scopeName,
  );
  const platformActiveDefinitions = useActivePlatformResourceDefinitions();
  const activeDefinitions = isPlatform ? platformActiveDefinitions : workspaceActiveDefinitions;
  const [versions, setVersions] = useState<ResourceVersionDetail[]>([]);
  const [resource, setResource] = useState<ResourceVersionDetail | null>(null);
  const [section, setSection] = useState<EditorSection>("data");
  const [editingField, setEditingField] = useState<InlineField>(null);
  const editorT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:resourceEditor.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    if (!scopeReady) return;
    if (isNew) {
      const workspaceDraft = createWorkspaceResourceDraft(scopeCode, scopeName);
      const draft = isPlatform ? {
        ...workspaceDraft,
        ownerWorkspaceCode: null,
        ownerWorkspaceName: null,
        scopeKind: "PLATFORM" as const,
        sourceKind: "PLATFORM" as const,
      } : workspaceDraft;
      setVersions([draft]);
      setResource(draft);
      return;
    }

    const storedVersions = readResourceVersionsFromUiSession(scopeCode, requestedResourceCode);
    const nextVersions = (storedVersions ?? sourceVersions).map(cloneVersion);
    const selected = nextVersions.find((record) => record.resourceVersion === requestedVersion)
      ?? nextVersions.find((record) => record.lifecycleStatus === "ACTIVE")
      ?? nextVersions[0]
      ?? null;
    setVersions(nextVersions);
    setResource(selected);
  }, [isNew, isPlatform, requestedResourceCode, requestedVersion, scopeCode, scopeName, scopeReady, sourceVersions]);

  const attributeErrors = useMemo(
    () => resource ? validateJsonValue(resource.attributesSchema, resource.attributes) : [],
    [resource],
  );
  const binaryErrors = useMemo(() => resource ? validateBinaries(resource) : [], [resource]);
  const structurallyValid = Boolean(
    resource
    && resource.name.trim()
    && resource.resourceCode.trim()
    && resource.definitionCode
    && !attributeErrors.length
    && !binaryErrors.length,
  );
  const publishable = Boolean(resource?.lifecycleStatus === "DRAFT" && structurallyValid);

  function updateResource(next: ResourceVersionDetail) {
    setResource(next);
    setVersions((current) => {
      const nextVersions = current.map((record) => (
        record.resourceVersion === next.resourceVersion ? next : record
      ));
      writeResourceVersionsToUiSession(scopeCode, next.resourceCode, nextVersions);
      return nextVersions;
    });
  }

  function patchDraft(patch: Partial<ResourceVersionDetail>) {
    if (!resource || resource.lifecycleStatus !== "DRAFT") return;
    updateResource({ ...resource, ...patch, updatedAt: new Date().toISOString() });
  }

  function selectDefinition(definitionCode: string) {
    if (!resource || resource.lifecycleStatus !== "DRAFT") return;
    const definition = activeDefinitions.find((record) => record.definitionCode === definitionCode);
    if (!definition) return;
    patchDraft({
      attributes: createJsonSchemaDraft7FormValue(definition.attributesSchema),
      attributesSchema: cloneJson(definition.attributesSchema),
      binaries: [],
      binarySchema: cloneJson(definition.binarySchema),
      definitionCode: definition.definitionCode,
      definitionName: definition.name,
      definitionVersion: definition.definitionVersion,
    });
  }

  function selectVersion(record: ResourceVersionDetail) {
    setResource(cloneVersion(record));
    setEditingField(null);
    setSearchParams({
      resourceCode: record.resourceCode,
      resourceVersion: record.resourceVersion,
    }, { replace: true });
  }

  function saveDraft() {
    if (!resource || resource.lifecycleStatus !== "DRAFT") return;
    const next = {
      ...resource,
      draftRevision: (resource.draftRevision ?? 0) + 1,
      updatedAt: new Date().toISOString(),
      validationIssueCount: structurallyValid ? 0 : Math.max(1, attributeErrors.length + binaryErrors.length),
      validationStatus: structurallyValid ? "VALID" as const : "INVALID" as const,
    };
    updateResource(next);
    toast.success(editorT("messages.saved", "Draft {{version}} saved", { version: next.resourceVersion }));
  }

  function publishDraft(target = resource) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    const targetAttributeErrors = validateJsonValue(target.attributesSchema, target.attributes);
    const targetBinaryErrors = validateBinaries(target);
    if (!target.name.trim() || !target.resourceCode.trim() || !target.definitionCode || targetAttributeErrors.length || targetBinaryErrors.length) return;
    const now = new Date().toISOString();
    const nextVersions = versions.map((record) => {
      if (record.resourceVersion === target.resourceVersion) {
        return {
          ...target,
          draftRevision: null,
          lifecycleStatus: "ACTIVE" as const,
          publishedAt: now,
          updatedAt: now,
          validationIssueCount: 0,
          validationStatus: "VALID" as const,
        };
      }
      if (record.lifecycleStatus === "ACTIVE") return { ...record, lifecycleStatus: "HISTORICAL" as const };
      return record;
    });
    const published = nextVersions.find((record) => record.resourceVersion === target.resourceVersion)!;
    writeResourceVersionsToUiSession(scopeCode, published.resourceCode, nextVersions);
    setVersions(nextVersions);
    setResource(published);
    if (isNew) {
      navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(published.resourceCode)}&resourceVersion=${encodeURIComponent(published.resourceVersion)}`, { replace: true });
    } else {
      setSearchParams({ resourceCode: published.resourceCode, resourceVersion: published.resourceVersion }, { replace: true });
    }
    toast.success(editorT("messages.published", "{{name}} {{version}} published", { name: published.name, version: published.resourceVersion }));
  }

  function discardDraft(target = resource) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    if (isNew) {
      navigate(basePath);
      return;
    }
    const remaining = versions.filter((record) => record.resourceVersion !== target.resourceVersion);
    const fallback = remaining.find((record) => record.lifecycleStatus === "ACTIVE") ?? remaining[0];
    writeResourceVersionsToUiSession(scopeCode, target.resourceCode, remaining);
    setVersions(remaining);
    if (fallback) selectVersion(fallback);
    toast.success(editorT("messages.discarded", "Draft {{version}} discarded", { version: target.resourceVersion }));
  }

  function openOrCreateDraft(record: ResourceVersionDetail) {
    const existingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (existingDraft) {
      selectVersion(existingDraft);
      setSection("data");
      toast.info(editorT("messages.openedDraft", "Existing draft {{version}} opened", { version: existingDraft.resourceVersion }));
      return;
    }

    const major = Math.max(...versions.map((item) => Number(item.resourceVersion.match(/^v(\d+)/)?.[1] ?? 0))) + 1;
    const now = new Date().toISOString();
    const draft: ResourceVersionDetail = {
      ...cloneVersion(record),
      createdAt: now,
      draftRevision: 1,
      lifecycleStatus: "DRAFT",
      ownerWorkspaceCode: isPlatform ? null : scopeCode,
      ownerWorkspaceName: isPlatform ? null : scopeName,
      publishedAt: null,
      resourceVersion: `v${major}-N4x`,
      scopeKind: isPlatform ? "PLATFORM" : "WORKSPACE",
      sourceKind: isPlatform
        ? "PLATFORM"
        : record.sourceKind === "INHERITED" || record.sourceKind === "PLATFORM" ? "OVERRIDE" : "LOCAL",
      updatedAt: now,
    };
    const nextVersions = [draft, ...versions];
    writeResourceVersionsToUiSession(scopeCode, draft.resourceCode, nextVersions);
    setVersions(nextVersions);
    setResource(draft);
    setSection("data");
    setSearchParams({ resourceCode: draft.resourceCode, resourceVersion: draft.resourceVersion }, { replace: true });
    toast.success(editorT("messages.createdDraft", "Draft {{version}} created", { version: draft.resourceVersion }));
  }

  function activateHistorical(record: ResourceVersionDetail) {
    const blockingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (blockingDraft) {
      toast.error(editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.resourceVersion }));
      return;
    }
    const now = new Date().toISOString();
    const nextVersions = versions.map((item) => {
      if (item.resourceVersion === record.resourceVersion) return { ...item, lifecycleStatus: "ACTIVE" as const, publishedAt: now, updatedAt: now };
      if (item.lifecycleStatus === "ACTIVE") return { ...item, lifecycleStatus: "HISTORICAL" as const };
      return item;
    });
    const activated = nextVersions.find((item) => item.resourceVersion === record.resourceVersion)!;
    writeResourceVersionsToUiSession(scopeCode, activated.resourceCode, nextVersions);
    setVersions(nextVersions);
    setResource(activated);
    setSearchParams({ resourceCode: activated.resourceCode, resourceVersion: activated.resourceVersion }, { replace: true });
    toast.success(editorT("messages.activated", "{{version}} activated", { version: record.resourceVersion }));
  }

  function handleVersionAction(record: ResourceVersionDetail, action: ResourceVersionHistoryAction) {
    if (action === "activate") activateHistorical(record);
    if (action === "createDraft") openOrCreateDraft(record);
    if (action === "discard") discardDraft(record);
    if (action === "edit" || action === "open") {
      selectVersion(record);
      setSection("data");
    }
    if (action === "publish") publishDraft(record);
  }

  function uploadBinary(binaryName: string, file: File) {
    if (!resource || resource.lifecycleStatus !== "DRAFT") return;
    if (resource.binaries.some((binary) => binary.binaryName === binaryName)) {
      toast.error(editorT("messages.duplicateBinary", "Binary {{name}} already exists", { name: binaryName }));
      return;
    }
    patchDraft({ binaries: [...resource.binaries, { binaryName }] });
    toast.success(editorT("messages.uploaded", "{{file}} uploaded as {{name}}", { file: file.name, name: binaryName }));
  }

  function replaceBinary(binary: ResourceBinaryMapping, file: File) {
    toast.success(editorT("messages.replaced", "{{name}} replaced with {{file}}", { file: file.name, name: binary.binaryName }));
  }

  function removeBinary(binary: ResourceBinaryMapping) {
    if (!resource || resource.lifecycleStatus !== "DRAFT") return;
    patchDraft({ binaries: resource.binaries.filter((item) => item.binaryName !== binary.binaryName) });
  }

  if (!scopeReady) {
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("emptyWorkspace", "Select a workspace to manage resources.")}</p>;
  }

  if (!resource) {
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("notFound", "Resource version not found.")}</p>;
  }

  const readOnly = resource.lifecycleStatus !== "DRAFT";
  const blockingDraft = versions.find((record) => record.lifecycleStatus === "DRAFT");
  const lifecycleTone = resource.lifecycleStatus === "ACTIVE" ? "success" : resource.lifecycleStatus === "DRAFT" ? "warning" : "info";
  const validationStatus = structurallyValid ? "VALID" : "INVALID";
  const tabItems: Array<{ count?: number; label: string; value: EditorSection }> = [
    { label: editorT("tabs.data", "Resource data"), value: "data" },
    { count: versions.length, label: editorT("tabs.history", "Version history"), value: "history" },
  ];

  return (
    <div className="min-w-0">
      <nav aria-label={editorT("breadcrumb.aria", "Breadcrumb")} className="mb-3 flex flex-wrap items-center gap-1 text-[12px] text-[var(--color-on-surface-variant)]">
        <Link className="hover:text-[var(--color-on-surface)]" to={basePath}>
          {isPlatform ? editorT("breadcrumb.platformResources", "Platform resources") : editorT("breadcrumb.resources", "Resources")}
        </Link>
        {!isNew ? <><span aria-hidden="true">/</span><span className="font-mono">{resource.resourceCode}</span></> : null}
      </nav>

      <header className="mb-4 border-b border-[var(--color-outline-variant)] pb-4">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              {isNew || editingField === "name" ? (
                <div className="flex min-w-[260px] max-w-xl flex-1 items-center gap-2">
                  <TextInput
                    aria-label={editorT("fields.name", "Resource name")}
                    autoFocus={editingField === "name"}
                    className="h-11 text-xl font-semibold"
                    disabled={readOnly}
                    placeholder={editorT("fields.namePlaceholder", "Resource name")}
                    value={resource.name}
                    onChange={(event) => patchDraft({ name: event.target.value })}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") setEditingField(null);
                    }}
                  />
                  {!isNew ? (
                    <Button aria-label={editorT("actions.finishName", "Finish editing name")} size="icon" variant="ghost" onClick={() => setEditingField(null)}>
                      <Check aria-hidden="true" size={16} />
                    </Button>
                  ) : null}
                </div>
              ) : (
                <div className="flex min-w-0 items-center gap-1">
                  <h1 className="truncate text-2xl font-semibold">{resource.name}</h1>
                  {!readOnly ? (
                    <Button aria-label={editorT("actions.editName", "Edit resource name")} size="icon-sm" variant="ghost" onClick={() => setEditingField("name")}>
                      <Pencil aria-hidden="true" size={14} />
                    </Button>
                  ) : null}
                </div>
              )}
              <Badge tone={lifecycleTone}>{resource.lifecycleStatus}</Badge>
              <Badge tone={validationStatus === "VALID" ? "success" : "error"}>{validationStatus}</Badge>
              <ResourceVersionSource inline record={resource} />
            </div>

            <div className="mt-2 max-w-3xl">
              {isNew || editingField === "description" ? (
                <div className="flex items-start gap-2">
                  <TextArea
                    aria-label={editorT("fields.description", "Description")}
                    autoFocus={editingField === "description"}
                    className="min-h-16"
                    disabled={readOnly}
                    placeholder={editorT("fields.descriptionPlaceholder", "Describe when agents and applications should use this resource")}
                    value={resource.description}
                    onChange={(event) => patchDraft({ description: event.target.value })}
                  />
                  {!isNew ? (
                    <Button aria-label={editorT("actions.finishDescription", "Finish editing description")} size="icon" variant="ghost" onClick={() => setEditingField(null)}>
                      <Check aria-hidden="true" size={16} />
                    </Button>
                  ) : null}
                </div>
              ) : (
                <div className="group flex items-start gap-1">
                  <p className="text-sm leading-6 text-[var(--color-on-surface-variant)]">{resource.description || editorT("fields.noDescription", "No description")}</p>
                  {!readOnly ? (
                    <Button aria-label={editorT("actions.editDescription", "Edit description")} className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100" size="icon-sm" variant="ghost" onClick={() => setEditingField("description")}>
                      <Pencil aria-hidden="true" size={14} />
                    </Button>
                  ) : null}
                </div>
              )}
            </div>

            <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-[12px] text-[var(--color-on-surface-variant)]">
              {isNew ? (
                <TextInput
                  aria-label={editorT("fields.code", "Resource code")}
                  className="h-8 w-56 font-mono text-[12px]"
                  placeholder="resource-code"
                  value={resource.resourceCode}
                  onChange={(event) => patchDraft({ resourceCode: event.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, "") })}
                />
              ) : <span className="font-mono font-medium text-[var(--color-on-surface)]">{resource.resourceCode}</span>}
              <span className="font-mono">{resource.resourceVersion}</span>
              {resource.draftRevision !== null ? <span className="font-mono">r{resource.draftRevision}</span> : null}
              {isNew ? (
                <SelectInput
                  aria-label={editorT("fields.definition", "Resource type")}
                  className="h-8 w-64 text-[12px]"
                  placeholder={editorT("fields.definitionPlaceholder", "Select resource type")}
                  size="sm"
                  value={resource.definitionCode}
                  onValueChange={(value) => {
                    if (value) selectDefinition(value);
                  }}
                >
                  {activeDefinitions.map((definition) => (
                    <SelectItem key={definition.definitionCode} value={definition.definitionCode}>
                      {definition.name} · {definition.definitionVersion}
                    </SelectItem>
                  ))}
                </SelectInput>
              ) : (
                <span>{resource.definitionName} <span className="font-mono">{resource.definitionVersion}</span></span>
              )}
              {resource.lifecycleStatus === "ACTIVE" ? (
                <ToggleSwitch
                  checked={resource.allowInheritance}
                  disabled={!isPlatform && (resource.sourceKind === "INHERITED" || resource.sourceKind === "PLATFORM")}
                  label={editorT("status.inheritance", "Allow inheritance")}
                  onCheckedChange={(allowInheritance) => updateResource({ ...resource, allowInheritance })}
                />
              ) : null}
            </div>
          </div>

          <div className="flex shrink-0 flex-wrap items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger aria-label={editorT("actions.more", "More resource actions")} className={buttonVariants({ size: "icon", variant: "ghost" })}>
                <MoreHorizontal aria-hidden="true" size={17} />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuItem onClick={() => setSection("history")}>{editorT("actions.viewHistory", "View version history")}</DropdownMenuItem>
                {resource.lifecycleStatus === "DRAFT" ? (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onClick={() => discardDraft()}>{editorT("actions.discard", "Discard draft")}</DropdownMenuItem>
                  </>
                ) : null}
              </DropdownMenuContent>
            </DropdownMenu>
            {resource.lifecycleStatus === "DRAFT" ? (
              <>
                <Button variant="outline" onClick={saveDraft}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.save", "Save draft")}
                </Button>
                <Button disabled={!publishable} title={!publishable ? editorT("actions.publishBlocked", "Resolve validation errors before publishing") : undefined} onClick={() => publishDraft()}>
                  <Send aria-hidden="true" size={15} />
                  {editorT("actions.publish", "Publish")}
                </Button>
              </>
            ) : resource.lifecycleStatus === "ACTIVE" ? (
              <Button onClick={() => openOrCreateDraft(resource)}>
                <Pencil aria-hidden="true" size={15} />
                {editorT("actions.edit", "Edit")}
              </Button>
            ) : (
              <Button
                disabled={Boolean(blockingDraft)}
                title={blockingDraft ? editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.resourceVersion }) : undefined}
                onClick={() => activateHistorical(resource)}
              >
                <Check aria-hidden="true" size={15} />
                {editorT("actions.activate", "Activate")}
              </Button>
            )}
          </div>
        </div>
      </header>

      <nav aria-label={editorT("tabs.aria", "Resource sections")} className="mb-4 flex min-w-0 gap-6 overflow-x-auto border-b border-[var(--color-outline-variant)]" role="tablist">
        {tabItems.map((item) => (
          <button
            aria-selected={section === item.value}
            className={cn(
              "relative -mb-px flex h-11 shrink-0 items-center gap-2 border-b-2 px-0.5 text-sm font-medium transition-colors",
              section === item.value
                ? "border-[var(--color-on-surface)] text-[var(--color-on-surface)]"
                : "border-transparent text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]",
            )}
            key={item.value}
            role="tab"
            type="button"
            onClick={() => setSection(item.value)}
          >
            {item.label}
            {item.count !== undefined ? <span className="rounded-full bg-[var(--color-surface-container)] px-1.5 py-0.5 font-mono text-[10px]">{item.count}</span> : null}
          </button>
        ))}
      </nav>

      <div role="tabpanel">
        {section === "data" ? (
          <div className={cn(
            "grid min-w-0 items-start gap-4",
            resource.definitionCode && resource.binarySchema.maxCount > 0
              ? "xl:grid-cols-[minmax(0,1.55fr)_minmax(320px,0.75fr)]"
              : "grid-cols-1",
          )}>
            <section className="min-w-0 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-4 sm:p-5">
              {resource.definitionCode ? (
                <JsonSchemaDraft7Form
                  errors={attributeErrors}
                  readOnly={readOnly}
                  schema={resource.attributesSchema}
                  value={resource.attributes}
                  onChange={(attributes) => patchDraft({ attributes })}
                />
              ) : (
                <div className="flex min-h-32 items-center justify-center text-sm text-[var(--color-on-surface-variant)]">
                  {editorT("emptyDefinition", "Select a resource type to configure its data.")}
                </div>
              )}
            </section>
            {resource.definitionCode && resource.binarySchema.maxCount > 0 ? (
              <aside className="min-w-0 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-4 xl:sticky xl:top-4">
                <ResourceBinaryEditor
                  binaries={resource.binaries}
                  readOnly={readOnly}
                  schema={resource.binarySchema}
                  onDownload={(binary) => toast.info(editorT("messages.downloaded", "Download started for {{name}}", { name: binary.binaryName }))}
                  onRemove={removeBinary}
                  onReplace={replaceBinary}
                  onUpload={uploadBinary}
                />
              </aside>
            ) : null}
          </div>
        ) : null}
        {section === "history" ? (
          <ResourceVersionHistory
            selectedVersion={resource.resourceVersion}
            versions={versions}
            onAction={handleVersionAction}
            onSelect={selectVersion}
          />
        ) : null}
      </div>
    </div>
  );
}
