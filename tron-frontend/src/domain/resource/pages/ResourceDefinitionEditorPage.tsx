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
  X,
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
import {
  ResourceDefinitionBinaryEditor,
  ResourceDefinitionSchemaEditor,
  ResourceDefinitionSource,
  ResourceDefinitionVersionHistory,
  validateResourceDefinitionBinarySchema,
} from "@/domain/resource/components";
import {
  usePlatformResourceDefinitionVersionDetails,
  useResourceDefinitionVersionDetails,
} from "@/domain/resource/hooks";
import {
  createWorkspaceResourceDefinitionDraft,
  readResourceDefinitionVersionsFromUiSession,
  writeResourceDefinitionVersionsToUiSession,
} from "@/domain/resource/mock";
import type {
  ResourceDefinitionAttributesSchema,
  ResourceDefinitionBinarySchema,
  ResourceDefinitionVersionDetail,
} from "@/domain/resource/model";
import {
  Badge,
  TextArea,
  TextInput,
  ToggleSwitch,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { cn } from "@/shared/lib/utils";

type EditorSection = "attributes" | "binaries" | "history";
type InlineField = "description" | "name" | null;

function cloneVersion(record: ResourceDefinitionVersionDetail) {
  return JSON.parse(JSON.stringify(record)) as ResourceDefinitionVersionDetail;
}

export function ResourceDefinitionEditorPage() {
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const isPlatform = location.pathname.startsWith("/settings/");
  const isNew = location.pathname.endsWith("/new");
  const basePath = isPlatform ? "/settings/resource-definitions" : "/resource-definitions";
  const scopeCode = isPlatform ? "PLATFORM" : activeWorkspace?.workspaceCode ?? "";
  const scopeName = isPlatform ? "Platform" : activeWorkspace?.name ?? "";
  const scopeReady = isPlatform || Boolean(activeWorkspace);
  const requestedDefinitionCode = searchParams.get("definitionCode") ?? (isPlatform ? "MYSQL_CONNECTION" : "PRODUCT_IMAGES");
  const requestedVersion = searchParams.get("definitionVersion") ?? (isPlatform ? "v4-a7K" : "v3-K7m");
  const workspaceSourceVersions = useResourceDefinitionVersionDetails(
    scopeCode,
    scopeName,
    requestedDefinitionCode,
  );
  const platformSourceVersions = usePlatformResourceDefinitionVersionDetails(requestedDefinitionCode);
  const sourceVersions = isPlatform ? platformSourceVersions : workspaceSourceVersions;
  const [versions, setVersions] = useState<ResourceDefinitionVersionDetail[]>([]);
  const [definition, setDefinition] = useState<ResourceDefinitionVersionDetail | null>(null);
  const [section, setSection] = useState<EditorSection>("attributes");
  const [editingField, setEditingField] = useState<InlineField>(null);
  const [schemaEditorValid, setSchemaEditorValid] = useState(true);
  const editorT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:resourceDefinitionEditor.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    if (!scopeReady) return;
    if (isNew) {
      const workspaceDraft = createWorkspaceResourceDefinitionDraft(scopeCode, scopeName);
      const draft = isPlatform ? {
        ...workspaceDraft,
        ownerWorkspaceCode: null,
        ownerWorkspaceName: null,
        scopeKind: "PLATFORM" as const,
        sourceKind: "PLATFORM" as const,
      } : workspaceDraft;
      setVersions([draft]);
      setDefinition(draft);
      return;
    }

    const storedVersions = readResourceDefinitionVersionsFromUiSession(scopeCode, requestedDefinitionCode);
    const nextVersions = (storedVersions ?? sourceVersions).map(cloneVersion);
    const selected = nextVersions.find((record) => record.definitionVersion === requestedVersion)
      ?? nextVersions[0]
      ?? null;
    setVersions(nextVersions);
    setDefinition(selected);
  }, [isNew, isPlatform, scopeCode, scopeName, scopeReady, sourceVersions]);

  const binaryErrors = useMemo(
    () => (definition ? validateResourceDefinitionBinarySchema(definition.binarySchema) : {}),
    [definition],
  );
  const structurallyValid = Boolean(
    definition
    && definition.name.trim()
    && definition.definitionCode.trim()
    && Object.keys(definition.attributesSchema.properties).length
    && !Object.keys(binaryErrors).length
    && schemaEditorValid,
  );
  const publishable = Boolean(
    definition
    && definition.lifecycleStatus === "DRAFT"
    && structurallyValid
    && (isNew || definition.validationStatus === "VALID"),
  );

  function updateDefinition(next: ResourceDefinitionVersionDetail) {
    setDefinition(next);
    setVersions((current) => {
      const nextVersions = current.map((record) => (
        record.definitionVersion === next.definitionVersion ? next : record
      ));
      writeResourceDefinitionVersionsToUiSession(scopeCode, next.definitionCode, nextVersions);
      return nextVersions;
    });
  }

  function patchDefinition(patch: Partial<ResourceDefinitionVersionDetail>) {
    if (!definition || definition.lifecycleStatus !== "DRAFT") return;
    const next = { ...definition, ...patch, updatedAt: new Date().toISOString() };
    updateDefinition(next);
  }

  function selectVersion(record: ResourceDefinitionVersionDetail) {
    setDefinition(cloneVersion(record));
    setEditingField(null);
    setSearchParams({
      definitionCode: record.definitionCode,
      definitionVersion: record.definitionVersion,
    }, { replace: true });
  }

  function saveDraft() {
    if (!definition || definition.lifecycleStatus !== "DRAFT") return;
    const next = {
      ...definition,
      draftRevision: (definition.draftRevision ?? 0) + 1,
      updatedAt: new Date().toISOString(),
      validationIssueCount: structurallyValid ? 0 : Math.max(1, definition.validationIssueCount),
      validationStatus: structurallyValid ? "VALID" as const : "INVALID" as const,
    };
    updateDefinition(next);
    toast.success(editorT("messages.saved", "Draft {{version}} saved", { version: next.definitionVersion }));
  }

  function publishDraft(target = definition) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    const targetIsCurrent = target.definitionVersion === definition?.definitionVersion;
    const targetStructurallyValid = Boolean(
      target.name.trim()
      && target.definitionCode.trim()
      && Object.keys(target.attributesSchema.properties).length
      && !Object.keys(validateResourceDefinitionBinarySchema(target.binarySchema)).length
      && (!targetIsCurrent || schemaEditorValid),
    );
    const targetPublishable = targetStructurallyValid && (isNew || target.validationStatus === "VALID");
    if (!targetPublishable) return;
    const now = new Date().toISOString();
    const nextVersions = versions.map((record) => {
      if (record.definitionVersion === target.definitionVersion) {
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
      if (record.lifecycleStatus === "ACTIVE") {
        return { ...record, lifecycleStatus: "HISTORICAL" as const };
      }
      return record;
    });
    const published = nextVersions.find((record) => record.definitionVersion === target.definitionVersion)!;
    writeResourceDefinitionVersionsToUiSession(scopeCode, published.definitionCode, nextVersions);
    setVersions(nextVersions);
    setDefinition(published);
    if (isNew) {
      navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(published.definitionCode)}&definitionVersion=${encodeURIComponent(published.definitionVersion)}`, { replace: true });
    } else {
      setSearchParams({ definitionCode: published.definitionCode, definitionVersion: published.definitionVersion }, { replace: true });
    }
    toast.success(editorT("messages.published", "{{name}} {{version}} published", { name: published.name, version: published.definitionVersion }));
  }

  function discardDraft(target = definition) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    if (isNew) {
      navigate(basePath);
      return;
    }
    const remaining = versions.filter((record) => record.definitionVersion !== target.definitionVersion);
    const fallback = remaining.find((record) => record.lifecycleStatus === "ACTIVE") ?? remaining[0];
    writeResourceDefinitionVersionsToUiSession(scopeCode, target.definitionCode, remaining);
    setVersions(remaining);
    if (fallback) selectVersion(fallback);
    toast.success(editorT("messages.discarded", "Draft {{version}} discarded", { version: target.definitionVersion }));
  }

  function openOrCreateDraft(record: ResourceDefinitionVersionDetail) {
    const existingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (existingDraft) {
      selectVersion(existingDraft);
      toast.info(editorT("messages.openedDraft", "Existing draft {{version}} opened", { version: existingDraft.definitionVersion }));
      return;
    }

    const major = Math.max(...versions.map((item) => Number(item.definitionVersion.match(/^v(\d+)/)?.[1] ?? 0))) + 1;
    const now = new Date().toISOString();
    const draft: ResourceDefinitionVersionDetail = {
      ...cloneVersion(record),
      createdAt: now,
      definitionVersion: `v${major}-N4x`,
      draftRevision: 1,
      lifecycleStatus: "DRAFT",
      publishedAt: null,
      ownerWorkspaceCode: isPlatform ? null : scopeCode,
      ownerWorkspaceName: isPlatform ? null : scopeName,
      scopeKind: isPlatform ? "PLATFORM" : "WORKSPACE",
      sourceKind: isPlatform
        ? "PLATFORM"
        : record.sourceKind === "INHERITED" || record.sourceKind === "PLATFORM" ? "OVERRIDE" : "LOCAL",
      updatedAt: now,
    };
    const nextVersions = [draft, ...versions];
    writeResourceDefinitionVersionsToUiSession(scopeCode, draft.definitionCode, nextVersions);
    setVersions(nextVersions);
    setDefinition(draft);
    setSearchParams({ definitionCode: draft.definitionCode, definitionVersion: draft.definitionVersion }, { replace: true });
    toast.success(editorT("messages.createdDraft", "Draft {{version}} created", { version: draft.definitionVersion }));
  }

  function activateHistorical(record: ResourceDefinitionVersionDetail) {
    const blockingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (blockingDraft) {
      toast.error(editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.definitionVersion }));
      return;
    }
    const now = new Date().toISOString();
    const nextVersions = versions.map((item) => {
      if (item.definitionVersion === record.definitionVersion) return { ...item, lifecycleStatus: "ACTIVE" as const, publishedAt: now, updatedAt: now };
      if (item.lifecycleStatus === "ACTIVE") return { ...item, lifecycleStatus: "HISTORICAL" as const };
      return item;
    });
    const activated = nextVersions.find((item) => item.definitionVersion === record.definitionVersion)!;
    writeResourceDefinitionVersionsToUiSession(scopeCode, activated.definitionCode, nextVersions);
    setVersions(nextVersions);
    setDefinition(activated);
    setSearchParams({ definitionCode: activated.definitionCode, definitionVersion: activated.definitionVersion }, { replace: true });
    toast.success(editorT("messages.activated", "{{version}} activated", { version: record.definitionVersion }));
  }

  function handleVersionAction(
    record: ResourceDefinitionVersionDetail,
    action: "activate" | "createDraft" | "discard" | "edit" | "open" | "publish",
  ) {
    if (action === "activate") activateHistorical(record);
    if (action === "createDraft") openOrCreateDraft(record);
    if (action === "discard") discardDraft(record);
    if (action === "edit" || action === "open") {
      selectVersion(record);
      setSection("attributes");
    }
    if (action === "publish") publishDraft(record);
  }

  if (!scopeReady) {
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("emptyWorkspace", "Select a workspace to manage resource definitions.")}</p>;
  }

  if (!definition) {
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("notFound", "Resource definition version not found.")}</p>;
  }

  const readOnly = definition.lifecycleStatus !== "DRAFT";
  const propertyCount = Object.keys(definition.attributesSchema.properties).length;
  const binaryCount = definition.binarySchema.maxCount > 0 ? definition.binarySchema.maxCount : 0;
  const blockingDraft = versions.find((record) => record.lifecycleStatus === "DRAFT");
  const lifecycleTone = definition.lifecycleStatus === "ACTIVE" ? "success" : definition.lifecycleStatus === "DRAFT" ? "warning" : "info";
  const tabItems: Array<{ count: number; label: string; value: EditorSection }> = [
    { count: propertyCount, label: editorT("tabs.attributes", "Structured attributes"), value: "attributes" },
    { count: binaryCount, label: editorT("tabs.binaries", "Binary constraints"), value: "binaries" },
    { count: versions.length, label: editorT("tabs.history", "Version history"), value: "history" },
  ];

  return (
    <div className="min-w-0">
      <nav aria-label={editorT("breadcrumb.aria", "Breadcrumb")} className="mb-3 flex flex-wrap items-center gap-1 text-[12px] text-[var(--color-on-surface-variant)]">
        {!isPlatform ? <><Link className="hover:text-[var(--color-on-surface)]" to="/resources">{editorT("breadcrumb.resources", "Resources")}</Link><span aria-hidden="true">/</span></> : null}
        <Link className="hover:text-[var(--color-on-surface)]" to={basePath}>
          {isPlatform ? editorT("breadcrumb.platformDefinitions", "Platform resource definitions") : editorT("breadcrumb.definitions", "Resource definitions")}
        </Link>
        {!isNew ? <><span aria-hidden="true">/</span><span className="font-mono">{definition.definitionCode}</span></> : null}
      </nav>

      <header className="mb-4 border-b border-[var(--color-outline-variant)] pb-4">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              {isNew || editingField === "name" ? (
                <div className="flex min-w-[280px] max-w-xl flex-1 items-center gap-2">
                  <TextInput
                    aria-label={editorT("fields.name", "Definition name")}
                    autoFocus={editingField === "name"}
                    className="h-11 text-xl font-semibold"
                    disabled={readOnly}
                    placeholder={editorT("fields.namePlaceholder", "Resource definition name")}
                    value={definition.name}
                    onChange={(event) => patchDefinition({ name: event.target.value })}
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
                  <h1 className="truncate text-2xl font-semibold">{definition.name}</h1>
                  {!readOnly ? (
                    <Button aria-label={editorT("actions.editName", "Edit definition name")} size="icon-sm" variant="ghost" onClick={() => setEditingField("name")}>
                      <Pencil aria-hidden="true" size={14} />
                    </Button>
                  ) : null}
                </div>
              )}
              <Badge tone={lifecycleTone}>{definition.lifecycleStatus}</Badge>
              <Badge tone={publishable || definition.validationStatus === "VALID" ? "success" : "error"}>
                {publishable || definition.validationStatus === "VALID" ? "VALID" : "INVALID"}
              </Badge>
              <ResourceDefinitionSource inline record={definition} />
            </div>

            <div className="mt-2 max-w-3xl">
              {isNew || editingField === "description" ? (
                <div className="flex items-start gap-2">
                  <TextArea
                    aria-label={editorT("fields.description", "Description")}
                    autoFocus={editingField === "description"}
                    className="min-h-16"
                    disabled={readOnly}
                    placeholder={editorT("fields.descriptionPlaceholder", "Describe when agents and applications should use this definition")}
                    value={definition.description}
                    onChange={(event) => patchDefinition({ description: event.target.value })}
                  />
                  {!isNew ? (
                    <Button aria-label={editorT("actions.finishDescription", "Finish editing description")} size="icon" variant="ghost" onClick={() => setEditingField(null)}>
                      <Check aria-hidden="true" size={16} />
                    </Button>
                  ) : null}
                </div>
              ) : (
                <div className="group flex items-start gap-1">
                  <p className="text-sm leading-6 text-[var(--color-on-surface-variant)]">{definition.description || editorT("fields.noDescription", "No description")}</p>
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
                  aria-label={editorT("fields.code", "Definition code")}
                  className="h-8 w-56 font-mono text-[12px]"
                  placeholder="DEFINITION_CODE"
                  value={definition.definitionCode}
                  onChange={(event) => patchDefinition({ definitionCode: event.target.value.toUpperCase().replace(/[^A-Z0-9_]/g, "") })}
                />
              ) : <span className="font-mono font-medium text-[var(--color-on-surface)]">{definition.definitionCode}</span>}
              <span className="font-mono">{definition.definitionVersion}</span>
              {definition.draftRevision !== null ? <span className="font-mono">r{definition.draftRevision}</span> : null}
              <ToggleSwitch checked={definition.enabled} disabled={readOnly} label={definition.enabled ? editorT("status.enabled", "Enabled") : editorT("status.disabled", "Disabled")} onCheckedChange={(checked) => patchDefinition({ enabled: checked })} />
              <ToggleSwitch checked={definition.allowInheritance} disabled={readOnly} label={editorT("status.inheritance", "Allow inheritance")} onCheckedChange={(checked) => patchDefinition({ allowInheritance: checked })} />
            </div>
          </div>

          <div className="flex shrink-0 flex-wrap items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger aria-label={editorT("actions.more", "More definition actions")} className={buttonVariants({ size: "icon", variant: "ghost" })}>
                <MoreHorizontal aria-hidden="true" size={17} />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuItem onClick={() => setSection("history")}>{editorT("actions.viewHistory", "View version history")}</DropdownMenuItem>
                {definition.lifecycleStatus === "DRAFT" ? (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onClick={() => discardDraft()}>{editorT("actions.discard", "Discard draft")}</DropdownMenuItem>
                  </>
                ) : null}
              </DropdownMenuContent>
            </DropdownMenu>
            {definition.lifecycleStatus === "DRAFT" ? (
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
            ) : definition.lifecycleStatus === "ACTIVE" ? (
              <Button onClick={() => openOrCreateDraft(definition)}>
                <Pencil aria-hidden="true" size={15} />
                {editorT("actions.edit", "Edit")}
              </Button>
            ) : (
              <Button
                disabled={Boolean(blockingDraft)}
                title={blockingDraft ? editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.definitionVersion }) : undefined}
                onClick={() => activateHistorical(definition)}
              >
                <Check aria-hidden="true" size={15} />
                {editorT("actions.activate", "Activate")}
              </Button>
            )}
          </div>
        </div>
      </header>

      <nav aria-label={editorT("tabs.aria", "Resource definition sections")} className="mb-4 flex min-w-0 gap-6 overflow-x-auto border-b border-[var(--color-outline-variant)]" role="tablist">
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
            <span className="rounded-full bg-[var(--color-surface-container)] px-1.5 py-0.5 font-mono text-[10px]">{item.count}</span>
          </button>
        ))}
      </nav>

      <div role="tabpanel">
        {section === "attributes" ? (
          <ResourceDefinitionSchemaEditor
            readOnly={readOnly}
            schema={definition.attributesSchema}
            onChange={(attributesSchema: ResourceDefinitionAttributesSchema) => patchDefinition({ attributesSchema })}
            onValidityChange={setSchemaEditorValid}
          />
        ) : null}
        {section === "binaries" ? (
          <ResourceDefinitionBinaryEditor
            readOnly={readOnly}
            schema={definition.binarySchema}
            onChange={(binarySchema: ResourceDefinitionBinarySchema) => patchDefinition({ binarySchema })}
          />
        ) : null}
        {section === "history" ? (
          <ResourceDefinitionVersionHistory
            selectedVersion={definition.definitionVersion}
            versions={versions}
            onAction={handleVersionAction}
            onSelect={selectVersion}
          />
        ) : null}
      </div>
    </div>
  );
}
