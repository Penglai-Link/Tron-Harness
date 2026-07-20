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
  Trash2,
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
import { adaptResourceVersionDetail } from "@/domain/resource/adapters";
import {
  activateResourceVersion,
  createResource,
  createResourceDraft,
  deleteResource,
  discardResourceDraft,
  downloadResourceBinary,
  getResource,
  pageResources,
  publishResourceDraft,
  removeResourceBinary,
  updateResource,
  updateResourceDraft,
  uploadResourceBinary,
} from "@/domain/resource/api";
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
  ConfirmDialog,
  SelectInput,
  TextArea,
  TextInput,
  ToggleSwitch,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { cn } from "@/shared/lib/utils";
import {
  createJsonSchemaDraft7FormValue,
  JsonSchemaDraft7Form,
  validateJsonValue,
} from "@/shared/ui/json-schema-draft7";

type EditorSection = "data" | "history";
type InlineField = "description" | "name" | null;
type ResourceEditMode = "COMMIT_ACTIVE" | "DRAFT" | "VIEW";

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function cloneVersion(record: ResourceVersionDetail) {
  return cloneJson(record);
}

function createUnsavedResource(
  scopeKind: "PLATFORM" | "WORKSPACE",
  scopeCode: string,
  scopeName: string,
): ResourceVersionDetail {
  const now = new Date().toISOString();
  return {
    allowInheritance: scopeKind === "PLATFORM",
    attributes: {},
    attributesSchema: {
      $schema: "http://json-schema.org/draft-07/schema#",
      additionalProperties: false,
      properties: {},
      required: [],
      type: "object",
    },
    binaries: [],
    binarySchema: {
      maxCount: 0,
      maxNameLength: 255,
      maxSizeBytes: 0,
      minCount: 0,
      namePattern: "",
      storageProfile: "@default",
    },
    createdAt: now,
    definitionCode: "",
    definitionName: "",
    definitionVersion: "",
    description: "",
    draftRevision: 0,
    lifecycleStatus: "DRAFT",
    name: "",
    ownerWorkspaceCode: scopeKind === "PLATFORM" ? null : scopeCode,
    ownerWorkspaceName: scopeKind === "PLATFORM" ? null : scopeName,
    publishedAt: null,
    resourceCode: "",
    resourceVersion: "",
    scopeKind,
    sourceKind: scopeKind === "PLATFORM" ? "PLATFORM" : "LOCAL",
    updatedAt: now,
    validationIssueCount: 0,
    validationStatus: "INVALID",
  };
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

async function loadApiResourceVersions(
  scopeKind: "PLATFORM" | "WORKSPACE",
  workspaceCode: string,
  resourceCode: string,
) {
  const request = {
    current: 1,
    definitionCode: null,
    definitionStatus: null,
    lifecycleStatus: null,
    resourceCode,
    scopeKind,
    search: null,
    size: 50,
    sourceKind: null,
    validationStatus: null,
    workspaceCode: scopeKind === "PLATFORM" ? null : workspaceCode,
  };
  const firstPage = await pageResources(request);
  const remainingPageCount = Math.ceil(firstPage.total / firstPage.size) - 1;
  const remainingPages = await Promise.all(Array.from({ length: remainingPageCount }, (_, index) => (
    pageResources({ ...request, current: index + 2 })
  )));
  const records = [firstPage, ...remainingPages].flatMap((page) => page.records);
  return Promise.all(records.map(async (record) => (
    adaptResourceVersionDetail(await getResource({
      resourceCode: record.resourceCode,
      resourceVersion: record.resourceVersion,
      scopeKind,
      workspaceCode: scopeKind === "PLATFORM" ? null : workspaceCode,
    }))
  )));
}

function requestErrorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Resource request failed";
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
  const isApi = getServiceDataSource("resource") === "api";
  const scopeKind = isPlatform ? "PLATFORM" as const : "WORKSPACE" as const;
  const isNew = location.pathname.endsWith("/new");
  const requestedResourceCode = searchParams.get("resourceCode") ?? (isPlatform ? "production-mysql" : "storefront-product-gallery");
  const requestedVersion = searchParams.get("resourceVersion") ?? (isPlatform ? "v3-A8n" : "v4-M7q");
  const workspaceSourceVersions = useResourceVersionDetails(
    scopeCode,
    scopeName,
    isApi ? "" : requestedResourceCode,
  );
  const platformSourceVersions = usePlatformResourceVersionDetails(isApi ? "" : requestedResourceCode);
  const sourceVersions = isPlatform ? platformSourceVersions : workspaceSourceVersions;
  // API mode owns the versions below. Do not let the compatibility data hook's
  // initialization rerun the route loader while an API mutation is in flight.
  const mockSourceVersions = isApi ? null : sourceVersions;
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
  const [editMode, setEditMode] = useState<ResourceEditMode>(isNew ? "COMMIT_ACTIVE" : "VIEW");
  const [baseBinaryNames, setBaseBinaryNames] = useState<string[]>([]);
  const [stagedBinaryFiles, setStagedBinaryFiles] = useState<Record<string, File>>({});
  const [removedBinaryNames, setRemovedBinaryNames] = useState<string[]>([]);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [committingActive, setCommittingActive] = useState(false);
  const apiLoadEpoch = useRef(0);
  const editorT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:resourceEditor.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    if (!scopeReady) return;
    if (isNew) {
      setEditMode("COMMIT_ACTIVE");
      setBaseBinaryNames([]);
      setStagedBinaryFiles({});
      setRemovedBinaryNames([]);
      if (isApi) {
        const draft = createUnsavedResource(scopeKind, scopeCode, scopeName);
        setVersions([]);
        setResource(draft);
        return;
      }
      const workspaceDraft = createWorkspaceResourceDraft(scopeCode, scopeName);
      const draft = isPlatform ? {
        ...workspaceDraft,
        ownerWorkspaceCode: null,
        ownerWorkspaceName: null,
        scopeKind: "PLATFORM" as const,
        sourceKind: "PLATFORM" as const,
      } : workspaceDraft;
      setVersions([]);
      setResource(draft);
      return;
    }

    if (isApi) {
      let cancelled = false;
      void reloadApiVersions(requestedResourceCode, requestedVersion)
        .catch((cause: unknown) => {
          if (!cancelled) toast.error(requestErrorMessage(cause));
        });
      return () => {
        cancelled = true;
        apiLoadEpoch.current += 1;
      };
    }

    const storedVersions = readResourceVersionsFromUiSession(scopeCode, requestedResourceCode);
    const nextVersions = (storedVersions ?? mockSourceVersions ?? []).map(cloneVersion);
    const selected = nextVersions.find((record) => record.resourceVersion === requestedVersion)
      ?? nextVersions.find((record) => record.lifecycleStatus === "ACTIVE")
      ?? nextVersions[0]
      ?? null;
    setVersions(nextVersions);
    setResource(selected);
    setEditMode(selected?.lifecycleStatus === "DRAFT" ? "DRAFT" : "VIEW");
    setBaseBinaryNames([]);
    setStagedBinaryFiles({});
    setRemovedBinaryNames([]);
  }, [isApi, isNew, isPlatform, mockSourceVersions, requestedResourceCode, requestedVersion, scopeCode, scopeKind, scopeName, scopeReady, toast]);

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
  // JSON Schema checks run locally for immediate form feedback.  The service is
  // still the source of truth for Variable references and binary-policy checks
  // that cannot be reproduced safely in the browser.
  const validationStatus = !structurallyValid
    ? "INVALID"
    : (isApi && resource?.resourceVersion ? resource.validationStatus : "VALID");
  const publishable = Boolean(
    resource?.lifecycleStatus === "DRAFT"
    && structurallyValid
    && validationStatus === "VALID",
  );

  function notifyServerValidation(
    status: ResourceVersionDetail["validationStatus"] | null,
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

  function replaceResourceState(next: ResourceVersionDetail) {
    setResource(next);
    setVersions((current) => {
      const nextVersions = current.map((record) => (
        record.resourceVersion === next.resourceVersion ? next : record
      ));
      if (!isApi) writeResourceVersionsToUiSession(scopeCode, next.resourceCode, nextVersions);
      return nextVersions;
    });
  }

  function patchResource(patch: Partial<ResourceVersionDetail>) {
    const isPersistedDraft = !isNew && resource?.lifecycleStatus === "DRAFT";
    if (!resource || (!isPersistedDraft && editMode !== "COMMIT_ACTIVE")) return;
    const next = {
      ...resource,
      ...patch,
      updatedAt: isApi ? resource.updatedAt : new Date().toISOString(),
    };
    if (isPersistedDraft) {
      replaceResourceState(next);
      return;
    }
    setResource(next);
  }

  function selectDefinition(definitionCode: string) {
    if (!resource || (editMode !== "DRAFT" && editMode !== "COMMIT_ACTIVE")) return;
    const definition = activeDefinitions.find((record) => record.definitionCode === definitionCode);
    if (!definition) return;
    if (editMode === "COMMIT_ACTIVE") {
      setBaseBinaryNames([]);
      setStagedBinaryFiles({});
      setRemovedBinaryNames([]);
    }
    patchResource({
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
    setEditMode(record.lifecycleStatus === "DRAFT" ? "DRAFT" : "VIEW");
    setBaseBinaryNames([]);
      setStagedBinaryFiles({});
      setRemovedBinaryNames([]);
    setSearchParams({
      resourceCode: record.resourceCode,
      resourceVersion: record.resourceVersion,
    }, { replace: true });
  }

  function navigateToApiResourceVersion(resourceCode: string, resourceVersion: string) {
    if (isNew) {
      navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(resourceCode)}&resourceVersion=${encodeURIComponent(resourceVersion)}`, { replace: true });
      return;
    }
    setSearchParams({ resourceCode, resourceVersion }, { replace: true });
  }

  async function reloadApiVersions(resourceCode: string, preferredVersion?: string) {
    const epoch = ++apiLoadEpoch.current;
    const nextVersions = await loadApiResourceVersions(scopeKind, scopeCode, resourceCode);
    if (epoch !== apiLoadEpoch.current) {
      return { nextVersions, selected: null, stale: true };
    }
    const selected = nextVersions.find((record) => record.resourceVersion === preferredVersion)
      ?? nextVersions.find((record) => record.lifecycleStatus === "ACTIVE")
      ?? nextVersions[0]
      ?? null;
    setVersions(nextVersions);
    setResource(selected);
    return { nextVersions, selected, stale: false };
  }

  async function saveDraft(
    target = resource,
    options: { notify?: boolean } = {},
  ): Promise<ResourceVersionDetail | null> {
    if (!target || target.lifecycleStatus !== "DRAFT") return null;
    if (isApi) {
      if (!target.resourceCode.trim() || !target.name.trim() || !target.definitionCode) {
        toast.error(editorT("messages.saveBlocked", "Enter a resource code, name, and resource type before saving."));
        return null;
      }
      try {
        const result = target.resourceVersion
          ? await updateResourceDraft({
            allowInheritance: target.allowInheritance,
            attributes: target.attributes,
            description: target.description,
            expectedDraftRevision: String(target.draftRevision ?? 0),
            name: target.name,
            resourceCode: target.resourceCode,
            resourceVersion: target.resourceVersion,
            scopeKind,
            workspaceCode: isPlatform ? null : scopeCode,
          })
          : await createResourceDraft({
            allowInheritance: target.allowInheritance,
            attributes: target.attributes,
            definitionCode: target.definitionCode,
            description: target.description,
            name: target.name,
            resourceCode: target.resourceCode,
            scopeKind,
            sourceResourceVersion: null,
            workspaceCode: isPlatform ? null : scopeCode,
          });
        const { selected } = await reloadApiVersions(result.resourceCode, result.resourceVersion);
        if (!selected) throw new Error("Resource draft was saved but could not be loaded");
        notifyServerValidation(result.validationStatus, result.validationErrors);
        if (!target.resourceVersion) {
          navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(selected.resourceCode)}&resourceVersion=${encodeURIComponent(selected.resourceVersion)}`, { replace: true });
        }
        if (options.notify !== false) {
          toast.success(editorT("messages.saved", "Draft {{version}} saved", { version: selected.resourceVersion }));
        }
        return selected;
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
        return null;
      }
    }

    const next = {
      ...target,
      draftRevision: (target.draftRevision ?? 0) + 1,
      updatedAt: new Date().toISOString(),
      validationIssueCount: structurallyValid ? 0 : Math.max(1, attributeErrors.length + binaryErrors.length),
      validationStatus: structurallyValid ? "VALID" as const : "INVALID" as const,
    };
    replaceResourceState(next);
    toast.success(editorT("messages.saved", "Draft {{version}} saved", { version: next.resourceVersion }));
    return next;
  }

  async function commitActive(target = resource) {
    if (!target) return;
    if (committingActive) return;
    const targetAttributeErrors = validateJsonValue(target.attributesSchema, target.attributes);
    const targetBinaryErrors = validateBinaries(target);
    if (!target.resourceCode.trim() || !target.name.trim() || !target.definitionCode) {
      toast.error(editorT("messages.saveBlocked", "Enter a resource code, name, and resource type before saving."));
      return;
    }
    if (targetAttributeErrors.length || targetBinaryErrors.length) {
      toast.error(editorT("messages.commitBlocked", "Resolve the form and binary validation errors before saving."));
      return;
    }
    const binaryFiles = Object.entries(stagedBinaryFiles).map(([binaryName, file]) => ({ binaryName, file }));
    if (isApi) {
      setCommittingActive(true);
      try {
        const request = {
          allowInheritance: target.allowInheritance,
          attributes: target.attributes,
          binaryFiles,
          description: target.description,
          expectedActiveResourceVersion: target.resourceVersion || null,
          name: target.name,
          removedBinaryNames,
          resourceCode: target.resourceCode,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        };
        const result = target.resourceVersion
          ? await updateResource({ ...request, definitionCode: null })
          : await createResource({ ...request, definitionCode: target.definitionCode });
        setEditMode("VIEW");
        setBaseBinaryNames([]);
        setStagedBinaryFiles({});
        setRemovedBinaryNames([]);
        navigateToApiResourceVersion(result.resourceCode, result.resourceVersion);
        toast.success(editorT(
          target.resourceVersion ? "messages.committed" : "messages.created",
          target.resourceVersion ? "New active version {{version}} saved" : "{{name}} {{version}} created",
          target.resourceVersion
            ? { version: result.resourceVersion }
            : { name: target.name, version: result.resourceVersion },
        ));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      } finally {
        setCommittingActive(false);
      }
      return;
    }

    const nextMajor = Math.max(
      0,
      ...versions.map((item) => Number(item.resourceVersion.match(/^v(\d+)/)?.[1] ?? 0)),
    ) + 1;
    const now = new Date().toISOString();
    const committed: ResourceVersionDetail = {
      ...cloneVersion(target),
      createdAt: now,
      draftRevision: null,
      lifecycleStatus: "ACTIVE",
      ownerWorkspaceCode: isPlatform ? null : scopeCode,
      ownerWorkspaceName: isPlatform ? null : scopeName,
      publishedAt: now,
      resourceVersion: `v${nextMajor}-N4x`,
      scopeKind,
      sourceKind: isPlatform
        ? "PLATFORM"
        : target.sourceKind === "INHERITED" || target.sourceKind === "PLATFORM" ? "OVERRIDE" : "LOCAL",
      updatedAt: now,
      validationIssueCount: 0,
      validationStatus: "VALID",
    };
    const previous = target.resourceVersion
      ? versions.map((item) => (
        item.resourceVersion === target.resourceVersion && item.lifecycleStatus === "ACTIVE"
          ? { ...item, lifecycleStatus: "HISTORICAL" as const }
          : item
      ))
      : versions.filter((item) => item.resourceVersion);
    const nextVersions = [committed, ...previous.filter((item) => item.resourceVersion !== committed.resourceVersion)];
    writeResourceVersionsToUiSession(scopeCode, committed.resourceCode, nextVersions);
    setVersions(nextVersions);
    setResource(committed);
    setEditMode("VIEW");
    setBaseBinaryNames([]);
    setStagedBinaryFiles({});
    setRemovedBinaryNames([]);
    if (isNew) {
      navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(committed.resourceCode)}&resourceVersion=${encodeURIComponent(committed.resourceVersion)}`, { replace: true });
    } else {
      setSearchParams({ resourceCode: committed.resourceCode, resourceVersion: committed.resourceVersion }, { replace: true });
    }
    toast.success(editorT("messages.committed", "New active version {{version}} saved", { version: committed.resourceVersion }));
  }

  async function publishDraft(target = resource) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    const targetAttributeErrors = validateJsonValue(target.attributesSchema, target.attributes);
    const targetBinaryErrors = validateBinaries(target);
    if (!target.name.trim() || !target.resourceCode.trim() || !target.definitionCode || targetAttributeErrors.length || targetBinaryErrors.length) return;
    if (isApi) {
      const saved = await saveDraft(target, { notify: false });
      if (!saved || saved.validationStatus !== "VALID" || saved.draftRevision === null) return;
      try {
        const result = await publishResourceDraft({
          expectedDraftRevision: String(saved.draftRevision),
          resourceCode: saved.resourceCode,
          resourceVersion: saved.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        const { selected } = await reloadApiVersions(result.resourceCode, result.resourceVersion);
        if (!selected) throw new Error("Published resource could not be loaded");
        if (isNew) {
          navigateToApiResourceVersion(selected.resourceCode, selected.resourceVersion);
        } else {
          setEditMode("VIEW");
        }
        toast.success(editorT("messages.published", "{{name}} {{version}} published", { name: selected.name, version: selected.resourceVersion }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      }
      return;
    }

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

  async function discardDraft(target = resource) {
    if (!target || target.lifecycleStatus !== "DRAFT") return;
    if (isApi) {
      if (!target.resourceVersion) {
        navigate(basePath);
        return;
      }
      const fallback = versions.find((record) => record.lifecycleStatus === "ACTIVE");
      try {
        await discardResourceDraft({
          expectedDraftRevision: String(target.draftRevision ?? 0),
          resourceCode: target.resourceCode,
          resourceVersion: target.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        if (fallback) {
          navigateToApiResourceVersion(fallback.resourceCode, fallback.resourceVersion);
        } else {
          navigate(basePath);
        }
        toast.success(editorT("messages.discarded", "Draft {{version}} discarded", { version: target.resourceVersion }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      }
      return;
    }

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

  async function openOrCreateDraft(record: ResourceVersionDetail) {
    if (committingActive) return;
    if (editMode === "COMMIT_ACTIVE" && (Object.keys(stagedBinaryFiles).length || removedBinaryNames.length)) {
      toast.warning(editorT(
        "messages.draftWithBinaryChangesBlocked",
        "Save the binary changes as an active version before saving unfinished content.",
      ));
      return;
    }
    const existingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT" && item.resourceVersion);
    if (existingDraft) {
      selectVersion(existingDraft);
      setSection("data");
      toast.info(editorT("messages.openedDraft", "Existing draft {{version}} opened", { version: existingDraft.resourceVersion }));
      return;
    }

    if (isApi) {
      try {
        const result = await createResourceDraft({
          allowInheritance: record.allowInheritance,
          attributes: record.attributes,
          definitionCode: record.resourceVersion ? null : record.definitionCode,
          description: record.description,
          name: record.name,
          resourceCode: record.resourceCode,
          scopeKind,
          sourceResourceVersion: record.resourceVersion || null,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        navigateToApiResourceVersion(result.resourceCode, result.resourceVersion);
        toast.success(editorT("messages.createdDraft", "Draft {{version}} created", { version: result.resourceVersion }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      }
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
    setEditMode("DRAFT");
    setBaseBinaryNames([]);
    setStagedBinaryFiles({});
    setRemovedBinaryNames([]);
    setSearchParams({ resourceCode: draft.resourceCode, resourceVersion: draft.resourceVersion }, { replace: true });
    toast.success(editorT("messages.createdDraft", "Draft {{version}} created", { version: draft.resourceVersion }));
  }

  async function activateHistorical(record: ResourceVersionDetail) {
    const blockingDraft = versions.find((item) => item.lifecycleStatus === "DRAFT");
    if (blockingDraft) {
      toast.error(editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.resourceVersion }));
      return;
    }
    if (isApi) {
      if (record.scopeKind !== scopeKind) {
        toast.error(editorT("messages.activateBlocked", "Only a locally owned historical version can be activated."));
        return;
      }
      const localActiveVersion = versions.find((item) => (
        item.lifecycleStatus === "ACTIVE" && item.scopeKind === scopeKind
      ))?.resourceVersion ?? null;
      try {
        const result = await activateResourceVersion({
          expectedActiveResourceVersion: localActiveVersion,
          resourceCode: record.resourceCode,
          resourceVersion: record.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        navigateToApiResourceVersion(result.resourceCode, result.resourceVersion);
        toast.success(editorT("messages.activated", "{{version}} activated", { version: result.resourceVersion }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
      }
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
    if (committingActive) return;
    if (action === "activate") activateHistorical(record);
    if (action === "createDraft") openOrCreateDraft(record);
    if (action === "discard") discardDraft(record);
    if (action === "edit" || action === "open") {
      selectVersion(record);
      setSection("data");
    }
    if (action === "publish") publishDraft(record);
  }

  async function uploadBinary(binaryName: string, file: File) {
    if (!resource) return;
    if (editMode === "COMMIT_ACTIVE") {
      if (resource.binaries.some((binary) => binary.binaryName === binaryName)) {
        const cause = new Error(editorT("messages.duplicateBinary", "Binary {{name}} already exists", { name: binaryName }));
        toast.error(cause.message);
        throw cause;
      }
      setStagedBinaryFiles((current) => ({ ...current, [binaryName]: file }));
      setRemovedBinaryNames((current) => current.filter((name) => name !== binaryName));
      patchResource({ binaries: [...resource.binaries, { binaryName, byteSize: file.size }] });
      toast.success(editorT("messages.binaryStaged", "{{file}} will be uploaded when you save", { file: file.name }));
      return;
    }
    if (resource.lifecycleStatus !== "DRAFT") return;
    if (isApi) {
      if (!resource.resourceVersion || resource.draftRevision === null) {
        const cause = new Error(editorT("messages.saveBeforeBinary", "Save the resource draft before changing binary data."));
        toast.error(cause.message);
        throw cause;
      }
      if (resource.binaries.some((binary) => binary.binaryName === binaryName)) {
        const cause = new Error(editorT("messages.duplicateBinary", "Binary {{name}} already exists", { name: binaryName }));
        toast.error(cause.message);
        throw cause;
      }
      try {
        const result = await uploadResourceBinary({
          binaryName,
          expectedDraftRevision: String(resource.draftRevision),
          file,
          resourceCode: resource.resourceCode,
          resourceVersion: resource.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        await reloadApiVersions(result.resourceCode, result.resourceVersion ?? resource.resourceVersion);
        notifyServerValidation(result.validationStatus, result.validationErrors);
        toast.success(editorT("messages.uploaded", "{{file}} uploaded as {{name}}", { file: file.name, name: binaryName }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
        throw cause;
      }
      return;
    }

    if (resource.binaries.some((binary) => binary.binaryName === binaryName)) {
      toast.error(editorT("messages.duplicateBinary", "Binary {{name}} already exists", { name: binaryName }));
      return;
    }
    patchResource({ binaries: [...resource.binaries, { binaryName }] });
    toast.success(editorT("messages.uploaded", "{{file}} uploaded as {{name}}", { file: file.name, name: binaryName }));
  }

  async function replaceBinary(binary: ResourceBinaryMapping, file: File) {
    if (resource && editMode === "COMMIT_ACTIVE") {
      setStagedBinaryFiles((current) => ({ ...current, [binary.binaryName]: file }));
      setRemovedBinaryNames((current) => current.filter((name) => name !== binary.binaryName));
      patchResource({
        binaries: resource.binaries.map((item) => (
          item.binaryName === binary.binaryName ? { ...item, byteSize: file.size } : item
        )),
      });
      toast.success(editorT("messages.binaryReplacementStaged", "{{name}} will be replaced when you save", { name: binary.binaryName }));
      return;
    }
    if (isApi) {
      if (!resource || resource.lifecycleStatus !== "DRAFT" || !resource.resourceVersion || resource.draftRevision === null) {
        const cause = new Error(editorT("messages.saveBeforeBinary", "Save the resource draft before changing binary data."));
        toast.error(cause.message);
        throw cause;
      }
      try {
        const result = await uploadResourceBinary({
          binaryName: binary.binaryName,
          expectedDraftRevision: String(resource.draftRevision),
          file,
          resourceCode: resource.resourceCode,
          resourceVersion: resource.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        await reloadApiVersions(result.resourceCode, result.resourceVersion ?? resource.resourceVersion);
        notifyServerValidation(result.validationStatus, result.validationErrors);
        toast.success(editorT("messages.replaced", "{{name}} replaced with {{file}}", { file: file.name, name: binary.binaryName }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
        throw cause;
      }
      return;
    }
    toast.success(editorT("messages.replaced", "{{name}} replaced with {{file}}", { file: file.name, name: binary.binaryName }));
  }

  async function removeBinary(binary: ResourceBinaryMapping) {
    if (!resource) return;
    if (editMode === "COMMIT_ACTIVE") {
      const wasPersisted = baseBinaryNames.includes(binary.binaryName);
      setStagedBinaryFiles((current) => {
        const next = { ...current };
        delete next[binary.binaryName];
        return next;
      });
      if (wasPersisted) {
        setRemovedBinaryNames((current) => (
          current.includes(binary.binaryName) ? current : [...current, binary.binaryName]
        ));
      }
      patchResource({ binaries: resource.binaries.filter((item) => item.binaryName !== binary.binaryName) });
      toast.success(editorT("messages.binaryRemovalStaged", "{{name}} will be removed when you save", { name: binary.binaryName }));
      return;
    }
    if (resource.lifecycleStatus !== "DRAFT") return;
    if (isApi) {
      if (!resource.resourceVersion || resource.draftRevision === null) {
        const cause = new Error(editorT("messages.saveBeforeBinary", "Save the resource draft before changing binary data."));
        toast.error(cause.message);
        throw cause;
      }
      try {
        const result = await removeResourceBinary({
          binaryName: binary.binaryName,
          expectedDraftRevision: String(resource.draftRevision),
          resourceCode: resource.resourceCode,
          resourceVersion: resource.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        await reloadApiVersions(result.resourceCode, result.resourceVersion ?? resource.resourceVersion);
        notifyServerValidation(result.validationStatus, result.validationErrors);
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
        throw cause;
      }
      return;
    }
    patchResource({ binaries: resource.binaries.filter((item) => item.binaryName !== binary.binaryName) });
  }

  async function downloadBinary(binary: ResourceBinaryMapping) {
    if (!resource) return;
    if (editMode === "COMMIT_ACTIVE" && Object.prototype.hasOwnProperty.call(stagedBinaryFiles, binary.binaryName)) {
      toast.info(editorT("messages.saveBeforeDownload", "Save this binary before downloading it."));
      return;
    }
    if (isApi) {
      try {
        const response = await downloadResourceBinary({
          binaryName: binary.binaryName,
          resourceCode: resource.resourceCode,
          resourceVersion: resource.resourceVersion,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
        downloadFile(response.blob, binary.binaryName, response.contentDisposition);
        toast.info(editorT("messages.downloaded", "Download started for {{name}}", { name: binary.binaryName }));
      } catch (cause) {
        toast.error(requestErrorMessage(cause));
        throw cause;
      }
      return;
    }
    toast.info(editorT("messages.downloaded", "Download started for {{name}}", { name: binary.binaryName }));
  }

  function beginDirectEdit(record: ResourceVersionDetail) {
    if (record.lifecycleStatus !== "ACTIVE") return;
    setResource(cloneVersion(record));
    setEditMode("COMMIT_ACTIVE");
    setBaseBinaryNames(record.binaries.map((binary) => binary.binaryName));
    setStagedBinaryFiles({});
    setRemovedBinaryNames([]);
    setEditingField(null);
    setSection("data");
  }

  function cancelDirectEdit() {
    setEditMode("VIEW");
    setBaseBinaryNames([]);
    setStagedBinaryFiles({});
    setRemovedBinaryNames([]);
    setEditingField(null);
    if (isNew) {
      navigate(basePath);
      return;
    }
    if (isApi && resource) {
      void reloadApiVersions(resource.resourceCode, resource.resourceVersion)
        .catch((cause: unknown) => toast.error(requestErrorMessage(cause)));
      return;
    }
    const original = resource && versions.find((record) => record.resourceVersion === resource.resourceVersion);
    if (original) selectVersion(original);
    else setEditMode("VIEW");
  }

  async function deleteCurrentResource() {
    if (!resource) return;
    setDeleting(true);
    try {
      const localActive = versions.find((record) => (
        record.scopeKind === scopeKind && record.lifecycleStatus === "ACTIVE"
      ));
      const localDraft = versions.find((record) => (
        record.scopeKind === scopeKind && record.lifecycleStatus === "DRAFT"
      ));
      if (isApi) {
        await deleteResource({
          expectedActiveResourceVersion: localActive?.resourceVersion ?? null,
          expectedDraftResourceVersion: localDraft?.resourceVersion ?? null,
          expectedDraftRevision: localDraft?.draftRevision === null || localDraft?.draftRevision === undefined
            ? null
            : String(localDraft.draftRevision),
          resourceCode: resource.resourceCode,
          scopeKind,
          workspaceCode: isPlatform ? null : scopeCode,
        });
      } else {
        const nextVersions = versions.filter((record) => (
          !(record.scopeKind === scopeKind && record.resourceCode === resource.resourceCode)
        ));
        writeResourceVersionsToUiSession(scopeCode, resource.resourceCode, nextVersions);
      }
      setDeleteOpen(false);
      toast.success(editorT("messages.deleted", "{{name}} deleted", { name: resource.name }));
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

  if (!resource) {
    return <p className="text-sm text-[var(--color-on-surface-variant)]">{editorT("notFound", "Resource version not found.")}</p>;
  }

  const isPersistedDraft = !isNew && resource.lifecycleStatus === "DRAFT";
  const readOnly = (!isPersistedDraft && editMode === "VIEW") || committingActive;
  const blockingDraft = versions.find((record) => record.lifecycleStatus === "DRAFT");
  const lifecycleTone = resource.lifecycleStatus === "ACTIVE" ? "success" : resource.lifecycleStatus === "DRAFT" ? "warning" : "info";
  const lifecycleLabel = isNew && editMode === "COMMIT_ACTIVE"
    ? editorT("status.new", "NEW")
    : resource.lifecycleStatus;
  const locallyOwned = resource.scopeKind === scopeKind;
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
                    onChange={(event) => patchResource({ name: event.target.value })}
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
              <Badge tone={lifecycleTone}>{lifecycleLabel}</Badge>
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
                    onChange={(event) => patchResource({ description: event.target.value })}
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
                  onChange={(event) => patchResource({ resourceCode: event.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, "") })}
                />
              ) : <span className="font-mono font-medium text-[var(--color-on-surface)]">{resource.resourceCode}</span>}
              {resource.resourceVersion ? <span className="font-mono">{resource.resourceVersion}</span> : null}
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
              <ToggleSwitch
                checked={resource.allowInheritance}
                disabled={readOnly}
                label={editorT("status.inheritance", "Allow inheritance")}
                onCheckedChange={(allowInheritance) => patchResource({ allowInheritance })}
              />
            </div>
          </div>

          <div className="flex shrink-0 flex-wrap items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger aria-label={editorT("actions.more", "More resource actions")} className={buttonVariants({ size: "icon", variant: "ghost" })} disabled={committingActive}>
                <MoreHorizontal aria-hidden="true" size={17} />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuItem onClick={() => setSection("history")}>{editorT("actions.viewHistory", "View version history")}</DropdownMenuItem>
                {isPersistedDraft ? (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onClick={() => discardDraft()}>{editorT("actions.discard", "Discard draft")}</DropdownMenuItem>
                  </>
                ) : null}
                {!isPersistedDraft ? (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onClick={() => { void openOrCreateDraft(resource); }}>
                      {editorT("actions.saveIncomplete", "Save unfinished content")}
                    </DropdownMenuItem>
                  </>
                ) : null}
                {locallyOwned && !isNew ? (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onClick={() => setDeleteOpen(true)}>
                      <Trash2 aria-hidden="true" size={14} />
                      {editorT("actions.delete", "Delete resource")}
                    </DropdownMenuItem>
                  </>
                ) : null}
              </DropdownMenuContent>
            </DropdownMenu>
            {editMode === "COMMIT_ACTIVE" ? (
              <>
                {!isNew ? (
                  <Button disabled={committingActive} variant="outline" onClick={cancelDirectEdit}>
                    {editorT("actions.cancel", "Cancel")}
                  </Button>
                ) : null}
                <Button disabled={!structurallyValid || committingActive} onClick={() => { void commitActive(); }}>
                  <Save aria-hidden="true" size={15} />
                  {isNew ? editorT("actions.create", "Create") : editorT("actions.saveActive", "Save")}
                </Button>
              </>
            ) : isPersistedDraft ? (
              <>
                <Button variant="outline" onClick={() => { void saveDraft(); }}>
                  <Save aria-hidden="true" size={15} />
                  {editorT("actions.save", "Save draft")}
                </Button>
                <Button disabled={!publishable} title={!publishable ? editorT("actions.publishBlocked", "Resolve validation errors before publishing") : undefined} onClick={() => { void publishDraft(); }}>
                  <Send aria-hidden="true" size={15} />
                  {editorT("actions.publish", "Publish")}
                </Button>
              </>
            ) : resource.lifecycleStatus === "ACTIVE" ? (
              <Button onClick={() => beginDirectEdit(resource)}>
                <Pencil aria-hidden="true" size={15} />
                {editorT("actions.edit", "Edit")}
              </Button>
            ) : (
              <Button
                disabled={Boolean(blockingDraft)}
                title={blockingDraft ? editorT("messages.activateBlocked", "Discard or publish draft {{version}} before activating history.", { version: blockingDraft.resourceVersion }) : undefined}
                onClick={() => { void activateHistorical(resource); }}
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
                  onChange={(attributes) => patchResource({ attributes })}
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
                  staged={editMode === "COMMIT_ACTIVE"}
                  onDownload={downloadBinary}
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
      <ConfirmDialog
        cancelText={editorT("actions.cancel", "Cancel")}
        confirmText={editorT("actions.delete", "Delete resource")}
        description={editorT("messages.deleteDescription", "Delete every version and binary mapping for this resource. This cannot be undone.")}
        open={deleteOpen}
        pending={deleting}
        title={editorT("messages.deleteTitle", "Delete {{name}}?", { name: resource.name })}
        onCancel={() => setDeleteOpen(false)}
        onConfirm={() => { void deleteCurrentResource(); }}
      />
    </div>
  );
}
