import {
  useEffect,
  useMemo,
  useRef,
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
import { adaptResourceDefinitionVersionDetail } from "@/domain/resource/adapters";
import {
  activateResourceDefinitionVersion,
  createResourceDefinitionDraft,
  deleteResourceDefinition,
  discardResourceDefinitionDraft,
  getResourceDefinition,
  pageResourceDefinitions,
  publishResourceDefinitionDraft,
  setResourceDefinitionEnabled,
  updateResourceDefinitionDraft,
} from "@/domain/resource/api";
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
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Badge,
  ConfirmDialog,
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

function createUnsavedDefinition(scopeKind: "PLATFORM" | "WORKSPACE", scopeCode: string): ResourceDefinitionVersionDetail {
  return {
    allowInheritance: scopeKind === "PLATFORM",
    attributesSchema: {
      $schema: "http://json-schema.org/draft-07/schema#",
      additionalProperties: false,
      properties: {},
      required: [],
      type: "object",
    },
    binarySchema: {
      maxCount: 0,
      maxNameLength: 255,
      maxSizeBytes: 0,
      minCount: 0,
      namePattern: "",
      storageProfile: "@default",
    },
    createdAt: "",
    effectiveBinaryPolicy: null,
    definitionCode: "",
    definitionVersion: "",
    description: "",
    draftRevision: null,
    enabled: true,
    lifecycleStatus: "DRAFT",
    name: "",
    ownerWorkspaceCode: scopeKind === "PLATFORM" ? null : scopeCode,
    ownerWorkspaceName: scopeKind === "PLATFORM" ? null : scopeCode,
    preset: false,
    publishedAt: null,
    scopeKind,
    sourceKind: scopeKind === "PLATFORM" ? "PLATFORM" : "LOCAL",
    updatedAt: "",
    validationIssueCount: 0,
    validationStatus: "INVALID",
  };
}

function apiBinarySchema(schema: ResourceDefinitionBinarySchema) {
  const { storageProfile: _storageProfile, ...value } = schema;
  return value;
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
  const isApi = getServiceDataSource("resource") === "api";
  const scopeKind = isPlatform ? "PLATFORM" as const : "WORKSPACE" as const;
  const requestedDefinitionCode = searchParams.get("definitionCode") ?? (isPlatform ? "MYSQL_CONNECTION" : "PRODUCT_IMAGES");
  const requestedVersion = searchParams.get("definitionVersion") ?? (isPlatform ? "v4-a7K" : "v3-K7m");
  const workspaceSourceVersions = useResourceDefinitionVersionDetails(
    isApi || isPlatform ? "" : scopeCode,
    isApi || isPlatform ? "" : scopeName,
    isApi || isPlatform ? "" : requestedDefinitionCode,
  );
  const platformSourceVersions = usePlatformResourceDefinitionVersionDetails(
    !isApi && isPlatform ? requestedDefinitionCode : "",
  );
  const sourceVersions = isPlatform ? platformSourceVersions : workspaceSourceVersions;
  // API mode owns the versions below. Do not let the compatibility data hook's
  // initialization rerun the route loader while an API mutation is in flight.
  const mockSourceVersions = isApi ? null : sourceVersions;
  const [versions, setVersions] = useState<ResourceDefinitionVersionDetail[]>([]);
  const [definition, setDefinition] = useState<ResourceDefinitionVersionDetail | null>(null);
  const [section, setSection] = useState<EditorSection>("attributes");
  const [editingField, setEditingField] = useState<InlineField>(null);
  const [schemaEditorValid, setSchemaEditorValid] = useState(true);
  const [activeSettingPending, setActiveSettingPending] = useState(false);
  const [deletePending, setDeletePending] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<{
    definitionCode: string;
    definitionVersion: string;
    name: string;
  } | null>(null);
  const apiLoadEpoch = useRef(0);
  const editorT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:resourceDefinitionEditor.${key}`, { defaultValue, ...options })
  );

  function apiScope() {
    return {
      scopeKind,
      workspaceCode: isPlatform ? null : scopeCode,
    };
  }

  async function loadApiVersions(definitionCode: string) {
    const request = {
      ...apiScope(),
      current: 1,
      definitionCode,
      definitionStatus: null,
      lifecycleStatus: null,
      search: null,
      size: 50,
      sourceKind: null,
      validationStatus: null,
    };
    const firstPage = await pageResourceDefinitions(request);
    const remainingPageCount = Math.ceil(firstPage.total / firstPage.size) - 1;
    const remainingPages = await Promise.all(Array.from({ length: remainingPageCount }, (_, index) => (
      pageResourceDefinitions({ ...request, current: index + 2 })
    )));
    const records = [firstPage, ...remainingPages].flatMap((page) => page.records);
    return Promise.all(records.map(async (record) => (
      adaptResourceDefinitionVersionDetail(await getResourceDefinition({
        ...apiScope(),
        definitionCode: record.definitionCode,
        definitionVersion: record.definitionVersion,
      }))
    )));
  }

  async function reloadApiVersions(definitionCode: string, preferredVersion?: string) {
    const epoch = ++apiLoadEpoch.current;
    const nextVersions = await loadApiVersions(definitionCode);
    if (epoch !== apiLoadEpoch.current) return null;
    const selected = nextVersions.find((record) => record.definitionVersion === preferredVersion)
      ?? nextVersions.find((record) => record.lifecycleStatus === "ACTIVE")
      ?? nextVersions[0]
      ?? null;
    setVersions(nextVersions);
    setDefinition(selected);
    return selected;
  }

  function showApiError(error: unknown, fallback: string) {
    toast.error(error instanceof Error ? error.message : fallback);
  }

  function notifyServerValidation(
    status: ResourceDefinitionVersionDetail["validationStatus"] | null,
    errors: ReadonlyArray<{ message: string; path: string }>,
  ) {
    if (status !== "INVALID") return;
    const firstIssue = errors[0];
    const issue = firstIssue
      ? `${firstIssue.path ? `${firstIssue.path}: ` : ""}${firstIssue.message}`
      : editorT("messages.serverValidationUnknown", "The server reported validation errors.");
    toast.warning(editorT(
      "messages.serverValidationFailed",
      "Draft was saved, but it cannot be published: {{issue}}",
      { issue },
    ));
  }

  useEffect(() => {
    if (!scopeReady) return;
    if (isApi) {
      if (isNew) {
        const draft = createUnsavedDefinition(scopeKind, scopeCode);
        setVersions([draft]);
        setDefinition(draft);
        return;
      }

      let cancelled = false;
      void reloadApiVersions(requestedDefinitionCode, requestedVersion)
        .catch((error: unknown) => {
          if (cancelled) return;
          showApiError(error, editorT("errors.load", "Resource definition could not be loaded"));
          setVersions([]);
          setDefinition(null);
        });
      return () => {
        cancelled = true;
        apiLoadEpoch.current += 1;
      };
    }

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
    const nextVersions = (storedVersions ?? mockSourceVersions ?? []).map(cloneVersion);
    const selected = nextVersions.find((record) => record.definitionVersion === requestedVersion)
      ?? nextVersions[0]
      ?? null;
    setVersions(nextVersions);
    setDefinition(selected);
  }, [
    isApi,
    isNew,
    isPlatform,
    requestedDefinitionCode,
    requestedVersion,
    scopeCode,
    scopeKind,
    scopeName,
    scopeReady,
    mockSourceVersions,
  ]);

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
  const validationStatus = !structurallyValid
    ? "INVALID"
    : (isApi && definition?.definitionVersion ? definition.validationStatus : "VALID");
  const publishable = Boolean(
    definition
    && definition.lifecycleStatus === "DRAFT"
    && structurallyValid
    && validationStatus === "VALID",
  );

  function updateDefinition(next: ResourceDefinitionVersionDetail) {
    setDefinition(next);
    setVersions((current) => {
      const nextVersions = current.map((record) => (
        record.definitionVersion === next.definitionVersion ? next : record
      ));
      if (!isApi) {
        writeResourceDefinitionVersionsToUiSession(scopeCode, next.definitionCode, nextVersions);
      }
      return nextVersions;
    });
  }

  function patchDefinition(patch: Partial<ResourceDefinitionVersionDetail>) {
    if (!definition || definition.lifecycleStatus !== "DRAFT") return;
    const next = {
      ...definition,
      ...patch,
      ...(isApi ? {} : { updatedAt: new Date().toISOString() }),
    };
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

  async function saveDraft(options: { navigateAfterSave?: boolean } = {}): Promise<ResourceDefinitionVersionDetail | null> {
    if (!definition || definition.lifecycleStatus !== "DRAFT") return null;
    if (isApi) {
      const definitionCode = definition.definitionCode.trim();
      if (!definitionCode || !definition.name.trim()) {
        toast.error(editorT("errors.required", "Definition code and name are required before saving"));
        return null;
      }
      try {
        if (definition.definitionVersion && definition.draftRevision === null) {
          toast.error(editorT("errors.revision", "Draft revision is required before saving"));
          return null;
        }
        const common = {
          allowInheritance: definition.allowInheritance,
          attributesSchema: definition.attributesSchema as Record<string, unknown>,
          binarySchema: apiBinarySchema(definition.binarySchema) as Record<string, unknown>,
          binaryStorageProfile: definition.binarySchema.storageProfile,
          definitionStatus: definition.enabled ? "ENABLED" as const : "DISABLED" as const,
          description: definition.description,
          name: definition.name,
        };
        const action = definition.definitionVersion
          ? await updateResourceDefinitionDraft({
            ...apiScope(),
            ...common,
            definitionCode,
            definitionVersion: definition.definitionVersion,
            expectedDraftRevision: String(definition.draftRevision),
          })
          : await createResourceDefinitionDraft({
            ...apiScope(),
            ...common,
            definitionCode,
            sourceDefinitionVersion: null,
          });
        const saved = await reloadApiVersions(definitionCode, action.definitionVersion);
        if (!saved) {
          toast.error(editorT("errors.notFound", "Saved definition version could not be reloaded"));
          return null;
        }
        notifyServerValidation(action.validationStatus, action.validationErrors);
        if (!definition.definitionVersion && options.navigateAfterSave !== false) {
          navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(saved.definitionCode)}&definitionVersion=${encodeURIComponent(saved.definitionVersion)}`, { replace: true });
        } else if (options.navigateAfterSave !== false) {
          setSearchParams({ definitionCode: saved.definitionCode, definitionVersion: saved.definitionVersion }, { replace: true });
        }
        toast.success(editorT("messages.saved", "Draft {{version}} saved", { version: saved.definitionVersion }));
        return saved;
      } catch (error) {
        showApiError(error, editorT("errors.save", "Resource definition draft could not be saved"));
        return null;
      }
    }

    const next = {
      ...definition,
      draftRevision: (definition.draftRevision ?? 0) + 1,
      updatedAt: new Date().toISOString(),
      validationIssueCount: structurallyValid ? 0 : Math.max(1, definition.validationIssueCount),
      validationStatus: structurallyValid ? "VALID" as const : "INVALID" as const,
    };
    updateDefinition(next);
    toast.success(editorT("messages.saved", "Draft {{version}} saved", { version: next.definitionVersion }));
    return next;
  }

  async function publishDraft(target = definition) {
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
    if (isApi) {
      try {
        const draft = targetIsCurrent ? await saveDraft({ navigateAfterSave: false }) : target;
        if (!draft || draft.validationStatus !== "VALID" || draft.draftRevision === null) return;
        const activation = await publishResourceDefinitionDraft({
          ...apiScope(),
          definitionCode: draft.definitionCode,
          definitionVersion: draft.definitionVersion,
          expectedDraftRevision: String(draft.draftRevision),
        });
        const published = await reloadApiVersions(draft.definitionCode, activation.definitionVersion);
        if (!published) {
          toast.error(editorT("errors.notFound", "Published definition version could not be reloaded"));
          return;
        }
        if (isNew) {
          navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(published.definitionCode)}&definitionVersion=${encodeURIComponent(published.definitionVersion)}`, { replace: true });
        } else {
          setSearchParams({ definitionCode: published.definitionCode, definitionVersion: published.definitionVersion }, { replace: true });
        }
        toast.success(editorT("messages.published", "{{name}} {{version}} published", { name: published.name, version: published.definitionVersion }));
      } catch (error) {
        showApiError(error, editorT("errors.publish", "Resource definition draft could not be published"));
      }
      return;
    }

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

  async function discardDraft(target = definition) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    if (isApi) {
      if (!target.definitionVersion) {
        navigate(basePath);
        return;
      }
      try {
        if (target.draftRevision === null) {
          toast.error(editorT("errors.revision", "Draft revision is required before discarding"));
          return;
        }
        await discardResourceDefinitionDraft({
          ...apiScope(),
          definitionCode: target.definitionCode,
          definitionVersion: target.definitionVersion,
          expectedDraftRevision: String(target.draftRevision),
        });
        const fallback = await reloadApiVersions(target.definitionCode);
        if (fallback) {
          setSearchParams({ definitionCode: fallback.definitionCode, definitionVersion: fallback.definitionVersion }, { replace: true });
        } else {
          navigate(basePath, { replace: true });
        }
        toast.success(editorT("messages.discarded", "Draft {{version}} discarded", { version: target.definitionVersion }));
      } catch (error) {
        showApiError(error, editorT("errors.discard", "Resource definition draft could not be discarded"));
      }
      return;
    }

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

  async function openOrCreateDraft(record: ResourceDefinitionVersionDetail) {
    const existingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (existingDraft) {
      selectVersion(existingDraft);
      toast.info(editorT("messages.openedDraft", "Existing draft {{version}} opened", { version: existingDraft.definitionVersion }));
      return;
    }

    if (isApi) {
      try {
        const action = await createResourceDefinitionDraft({
          ...apiScope(),
          allowInheritance: record.allowInheritance,
          attributesSchema: record.attributesSchema as Record<string, unknown>,
          binarySchema: apiBinarySchema(record.binarySchema) as Record<string, unknown>,
          binaryStorageProfile: record.binarySchema.storageProfile,
          definitionCode: record.definitionCode,
          definitionStatus: record.enabled ? "ENABLED" : "DISABLED",
          description: record.description,
          name: record.name,
          sourceDefinitionVersion: record.definitionVersion,
        });
        const draft = await reloadApiVersions(record.definitionCode, action.definitionVersion);
        if (!draft) {
          toast.error(editorT("errors.notFound", "New draft could not be reloaded"));
          return;
        }
        setSearchParams({ definitionCode: draft.definitionCode, definitionVersion: draft.definitionVersion }, { replace: true });
        toast.success(editorT("messages.createdDraft", "Draft {{version}} created", { version: draft.definitionVersion }));
      } catch (error) {
        showApiError(error, editorT("errors.createDraft", "Resource definition draft could not be created"));
      }
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

  async function activateHistorical(record: ResourceDefinitionVersionDetail) {
    const blockingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (blockingDraft) {
      toast.error(editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.definitionVersion }));
      return;
    }
    if (isApi) {
      if (record.scopeKind !== scopeKind) {
        toast.error(editorT("messages.activateBlocked", "Only a locally owned historical version can be activated."));
        return;
      }
      const localActiveDefinitionVersion = versions.find((item) => (
        item.lifecycleStatus === "ACTIVE" && item.scopeKind === scopeKind
      ))?.definitionVersion ?? null;
      try {
        const activation = await activateResourceDefinitionVersion({
          ...apiScope(),
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          expectedActiveDefinitionVersion: localActiveDefinitionVersion,
        });
        const activated = await reloadApiVersions(record.definitionCode, activation.definitionVersion);
        if (!activated) {
          toast.error(editorT("errors.notFound", "Activated definition version could not be reloaded"));
          return;
        }
        setSearchParams({ definitionCode: activated.definitionCode, definitionVersion: activated.definitionVersion }, { replace: true });
        toast.success(editorT("messages.activated", "{{version}} activated", { version: activated.definitionVersion }));
      } catch (error) {
        showApiError(error, editorT("errors.activate", "Resource definition version could not be activated"));
      }
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

  async function setActiveDefinitionEnabled(enabled: boolean) {
    if (!definition || definition.lifecycleStatus !== "ACTIVE" || definition.enabled === enabled) return;
    if (definition.scopeKind !== scopeKind) {
      toast.error(editorT("messages.setEnabledBlocked", "Only a locally owned active definition can be enabled or disabled."));
      return;
    }

    if (!isApi) {
      updateDefinition({
        ...definition,
        enabled,
        updatedAt: new Date().toISOString(),
      });
      toast.success(editorT(
        enabled ? "messages.enabled" : "messages.disabled",
        enabled ? "{{name}} enabled" : "{{name}} disabled",
        { name: definition.name },
      ));
      return;
    }

    setActiveSettingPending(true);
    try {
      const result = await setResourceDefinitionEnabled({
        ...apiScope(),
        definitionCode: definition.definitionCode,
        enabled,
        expectedActiveDefinitionVersion: definition.definitionVersion,
      });
      const updated = await reloadApiVersions(result.definitionCode, result.definitionVersion);
      if (!updated) {
        toast.error(editorT("errors.notFound", "Updated definition could not be reloaded"));
        return;
      }
      toast.success(editorT(
        enabled ? "messages.enabled" : "messages.disabled",
        enabled ? "{{name}} enabled" : "{{name}} disabled",
        { name: updated.name },
      ));
    } catch (error) {
      showApiError(error, editorT("errors.setEnabled", "Resource definition enabled state could not be updated"));
    } finally {
      setActiveSettingPending(false);
    }
  }

  async function deleteDefinition() {
    const target = deleteTarget;
    if (!target) return;

    setDeletePending(true);
    try {
      const result = await deleteResourceDefinition({
        ...apiScope(),
        definitionCode: target.definitionCode,
        expectedActiveDefinitionVersion: target.definitionVersion,
      });
      if (!result.deleted) {
        throw new Error(editorT("errors.delete", "Resource definition could not be deleted"));
      }
      setDeleteTarget(null);
      toast.success(editorT("messages.deleted", "{{name}} deleted", { name: target.name }));
      navigate(basePath, { replace: true });
    } catch (error) {
      showApiError(error, editorT("errors.delete", "Resource definition could not be deleted"));
    } finally {
      setDeletePending(false);
    }
  }

  function handleVersionAction(
    record: ResourceDefinitionVersionDetail,
    action: "activate" | "createDraft" | "discard" | "edit" | "open" | "publish",
  ) {
    if (action === "activate") void activateHistorical(record);
    if (action === "createDraft") void openOrCreateDraft(record);
    if (action === "discard") void discardDraft(record);
    if (action === "edit" || action === "open") {
      selectVersion(record);
      setSection("attributes");
    }
    if (action === "publish") void publishDraft(record);
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
  const ownsActiveDefinition = definition.lifecycleStatus === "ACTIVE" && definition.scopeKind === scopeKind;
  const canChangeEnabled = definition.lifecycleStatus === "DRAFT" || ownsActiveDefinition;
  const canDeleteDefinition = isApi
    && ownsActiveDefinition
    && !definition.preset
    && !definition.enabled
    && !blockingDraft;
  const lifecycleTone = definition.lifecycleStatus === "ACTIVE" ? "success" : definition.lifecycleStatus === "DRAFT" ? "warning" : "info";
  const tabItems: Array<{ count: number; label: string; value: EditorSection }> = [
    { count: propertyCount, label: editorT("tabs.attributes", "Information fields"), value: "attributes" },
    { count: binaryCount, label: editorT("tabs.binaries", "File rules"), value: "binaries" },
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
              <Badge tone={validationStatus === "VALID" ? "success" : "error"}>
                {validationStatus}
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
              <ToggleSwitch
                checked={definition.enabled}
                disabled={!canChangeEnabled || activeSettingPending}
                label={definition.enabled ? editorT("status.enabled", "Enabled") : editorT("status.disabled", "Disabled")}
                onCheckedChange={(checked) => {
                  if (definition.lifecycleStatus === "DRAFT") {
                    patchDefinition({ enabled: checked });
                    return;
                  }
                  void setActiveDefinitionEnabled(checked);
                }}
              />
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
                    <DropdownMenuItem variant="destructive" onClick={() => void discardDraft()}>{editorT("actions.discard", "Discard draft")}</DropdownMenuItem>
                  </>
                ) : null}
                {canDeleteDefinition ? (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      variant="destructive"
                      onClick={() => setDeleteTarget({
                        definitionCode: definition.definitionCode,
                        definitionVersion: definition.definitionVersion,
                        name: definition.name,
                      })}
                    >
                      {editorT("actions.delete", "Delete definition")}
                    </DropdownMenuItem>
                  </>
                ) : null}
              </DropdownMenuContent>
            </DropdownMenu>
            {definition.lifecycleStatus === "DRAFT" ? (
              <>
                <Button variant="outline" onClick={() => void saveDraft()}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.save", "Save draft")}
                </Button>
                <Button disabled={!publishable} title={!publishable ? editorT("actions.publishBlocked", "Resolve validation errors before publishing") : undefined} onClick={() => void publishDraft()}>
                  <Send aria-hidden="true" size={15} />
                  {editorT("actions.publish", "Publish")}
                </Button>
              </>
            ) : definition.lifecycleStatus === "ACTIVE" ? (
              <Button onClick={() => void openOrCreateDraft(definition)}>
                <Pencil aria-hidden="true" size={15} />
                {editorT("actions.edit", "Edit")}
              </Button>
            ) : (
              <Button
                disabled={Boolean(blockingDraft)}
                title={blockingDraft ? editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.definitionVersion }) : undefined}
                onClick={() => void activateHistorical(definition)}
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
      <ConfirmDialog
        ariaLabel={editorT("deleteDialog.aria", "Delete resource definition confirmation")}
        cancelText={editorT("deleteDialog.cancel", "Cancel")}
        confirmText={deletePending
          ? editorT("deleteDialog.deleting", "Deleting...")
          : editorT("actions.delete", "Delete definition")}
        description={deleteTarget ? editorT(
          "deleteDialog.description",
          "Delete {{name}} and all of its version history? This cannot be undone.",
          { name: deleteTarget.name },
        ) : undefined}
        open={deleteTarget !== null}
        pending={deletePending}
        title={editorT("deleteDialog.title", "Delete resource definition?")}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => void deleteDefinition()}
      />
    </div>
  );
}
