import {
  useEffect,
  useState,
} from "react";
import {
  Navigate,
  useNavigate,
  useSearchParams,
} from "react-router";

import {
  fetchPlatformOpenApp,
  fetchPlatformOpenAppCatalog,
  replacePlatformOpenAppCapabilityGrants,
  replacePlatformOpenAppUserGrants,
  updatePlatformOpenApp,
  type UpdateOpenAppPayload,
} from "@/domain/open-app/api";
import {
  capabilityGrantKey,
  emptyMetadataDraft,
  normalizeUserCodesInput,
  OpenAppCapabilityGrantEditorSection,
  OpenAppMetadataFormFields,
  OpenAppUserGrantEditorSection,
  type MetadataDraft,
  useOpenAppPageT,
} from "@/domain/open-app/components";
import {
  getPlatformOpenAppMock,
  listPlatformOpenAppCatalogMock,
  replacePlatformOpenAppCapabilityGrantsMock,
  replacePlatformOpenAppUserGrantsMock,
  updatePlatformOpenAppMock,
} from "@/domain/open-app/mock";
import type {
  ManagedOpenAppDetail,
  OpenApiCapabilityCatalogGroup,
} from "@/domain/open-app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Button,
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

export function EditPlatformOpenAppPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const pageT = useOpenAppPageT();
  const dataSource = getServiceDataSource("admin");
  const openAppCode = searchParams.get("openAppCode") ?? "";
  const [detail, setDetail] = useState<ManagedOpenAppDetail | null>(null);
  const [catalog, setCatalog] = useState<OpenApiCapabilityCatalogGroup[]>([]);
  const [metadataDraft, setMetadataDraft] = useState<MetadataDraft>(emptyMetadataDraft);
  const [userCodesInput, setUserCodesInput] = useState("");
  const [selectedGrantKeys, setSelectedGrantKeys] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [savingMetadata, setSavingMetadata] = useState(false);
  const [savingUsers, setSavingUsers] = useState(false);
  const [savingCapabilities, setSavingCapabilities] = useState(false);

  function syncEditors(nextDetail: ManagedOpenAppDetail) {
    setMetadataDraft({
      description: nextDetail.description ?? "",
      name: nextDetail.name,
      status: nextDetail.status,
    });
    setUserCodesInput(nextDetail.userGrants.map((grant) => grant.userCode).join("\n"));
    setSelectedGrantKeys(
      nextDetail.capabilityGrants.map((grant) => (
        capabilityGrantKey(grant.capabilityCode, grant.operationCode)
      )),
    );
  }

  async function loadPageData() {
    if (!openAppCode) return;
    setLoading(true);
    try {
      const [detailResult, catalogResult] = dataSource === "api"
        ? await Promise.allSettled([
          fetchPlatformOpenApp(openAppCode),
          fetchPlatformOpenAppCatalog(),
        ])
        : await Promise.allSettled([
          Promise.resolve(getPlatformOpenAppMock(openAppCode)),
          Promise.resolve(listPlatformOpenAppCatalogMock()),
        ]);

      if (detailResult.status === "fulfilled") {
        setDetail(detailResult.value);
        syncEditors(detailResult.value);
      } else {
        setDetail(null);
        toast.error(detailResult.reason instanceof Error ? detailResult.reason.message : pageT("errors.detailLoad", "Open-app details could not be loaded"));
      }

      if (catalogResult.status === "fulfilled") {
        setCatalog(catalogResult.value);
      } else {
        setCatalog([]);
        toast.error(catalogResult.reason instanceof Error ? catalogResult.reason.message : pageT("errors.catalogLoad", "OpenAPI capability catalog could not be loaded"));
      }
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadPageData();
  }, [dataSource, openAppCode]);

  if (!openAppCode) {
    return <Navigate replace to="/settings/openapi" />;
  }

  async function handleSaveMetadata() {
    if (!detail) return;
    const payload: UpdateOpenAppPayload = {
      description: metadataDraft.description,
      name: metadataDraft.name,
      status: metadataDraft.status,
    };
    setSavingMetadata(true);
    try {
      const nextDetail = dataSource === "api"
        ? await updatePlatformOpenApp(detail.openAppCode, payload)
        : updatePlatformOpenAppMock(detail.openAppCode, payload);
      setDetail(nextDetail);
      syncEditors(nextDetail);
      toast.success(pageT("messages.metadataSaved", "{{name}} saved", { name: nextDetail.name }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.metadataSave", "Open-app could not be updated"));
    } finally {
      setSavingMetadata(false);
    }
  }

  async function handleSaveUserGrants() {
    if (!detail) return;
    setSavingUsers(true);
    try {
      const nextDetail = dataSource === "api"
        ? await replacePlatformOpenAppUserGrants(detail.openAppCode, {
          userCodes: normalizeUserCodesInput(userCodesInput),
        })
        : replacePlatformOpenAppUserGrantsMock(detail.openAppCode, {
          userCodes: normalizeUserCodesInput(userCodesInput),
        });
      setDetail(nextDetail);
      syncEditors(nextDetail);
      toast.success(pageT("messages.userGrantsSaved", "User grants updated"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.userGrantSave", "User grants could not be updated"));
    } finally {
      setSavingUsers(false);
    }
  }

  async function handleSaveCapabilityGrants() {
    if (!detail) return;
    setSavingCapabilities(true);
    try {
      const grants = selectedGrantKeys.map((key) => {
        const [capabilityCode, operationCode] = key.split(":");
        return { capabilityCode, operationCode };
      });
      const nextDetail = dataSource === "api"
        ? await replacePlatformOpenAppCapabilityGrants(detail.openAppCode, { grants })
        : replacePlatformOpenAppCapabilityGrantsMock(detail.openAppCode, { grants });
      setDetail(nextDetail);
      syncEditors(nextDetail);
      toast.success(pageT("messages.capabilityGrantsSaved", "Capability grants updated"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.capabilityGrantSave", "Capability grants could not be updated"));
    } finally {
      setSavingCapabilities(false);
    }
  }

  function toggleCapabilityGrant(capabilityCode: string, operationCode: string) {
    const key = capabilityGrantKey(capabilityCode, operationCode);
    setSelectedGrantKeys((current) => {
      if (current.includes(key)) {
        return current.filter((item) => item !== key);
      }
      return [...current, key].sort();
    });
  }

  function setCapabilityGrantGroupSelection(
    capabilityCode: string,
    operationCodes: string[],
    selected: boolean,
  ) {
    const operationKeys = operationCodes.map((operationCode) => capabilityGrantKey(capabilityCode, operationCode));
    setSelectedGrantKeys((current) => {
      const next = new Set(current);
      if (selected) {
        operationKeys.forEach((key) => next.add(key));
      } else {
        operationKeys.forEach((key) => next.delete(key));
      }
      return Array.from(next).sort();
    });
  }

  if (loading && !detail) {
    return (
      <>
        <PageHeader
          breadcrumbs={[
            { label: pageT("breadcrumbs.list", "OpenAPI"), href: "/settings/openapi" },
            { label: pageT("breadcrumbs.loading", "Loading") },
          ]}
          title={pageT("title.edit", "Edit Open App")}
        />
        <SurfaceCard>{pageT("edit.loading", "Loading open-app editor...")}</SurfaceCard>
      </>
    );
  }

  if (!detail) {
    return (
      <>
        <PageHeader
          breadcrumbs={[
            { label: pageT("breadcrumbs.list", "OpenAPI"), href: "/settings/openapi" },
            { label: pageT("breadcrumbs.notFound", "Not Found") },
          ]}
          title={pageT("title.edit", "Edit Open App")}
        />
        <SurfaceCard>{pageT("edit.notFound", "Open-app not found.")}</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: pageT("breadcrumbs.list", "OpenAPI"), href: "/settings/openapi" },
          { label: detail.name, href: `/settings/openapi/detail?openAppCode=${encodeURIComponent(detail.openAppCode)}` },
          { label: pageT("breadcrumbs.edit", "Edit") },
        ]}
        title={pageT("edit.titleWithName", "Edit {{name}}", { name: detail.name })}
        actions={(
          <>
            <Button
              type="button"
              variant="secondary"
              onClick={() => navigate(`/settings/openapi/detail?openAppCode=${encodeURIComponent(detail.openAppCode)}`)}
            >
              {pageT("actions.cancel", "Cancel")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-6">
        <SurfaceCard className="space-y-6">
          <div>
            <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">
              {pageT("metadata.title", "Open App Metadata")}
            </h2>
          </div>

          <OpenAppMetadataFormFields draft={metadataDraft} onChange={setMetadataDraft} />

          <div className="flex flex-wrap gap-2">
            <Button
              disabled={savingMetadata || !metadataDraft.name.trim()}
              aria-busy={savingMetadata}
              onClick={() => void handleSaveMetadata()}
            >
              {savingMetadata ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </div>
        </SurfaceCard>

        <OpenAppUserGrantEditorSection
          disabled={savingUsers}
          value={userCodesInput}
          onChange={setUserCodesInput}
          footer={(
            <div className="flex flex-wrap gap-2">
              <Button
                disabled={savingUsers}
                aria-busy={savingUsers}
                onClick={() => void handleSaveUserGrants()}
              >
                {savingUsers ? pageT("actions.saving", "Saving...") : pageT("actions.saveUsers", "Grant")}
              </Button>
            </div>
          )}
        />

        <OpenAppCapabilityGrantEditorSection
          catalog={catalog}
          disabled={savingCapabilities}
          selectedGrantKeys={selectedGrantKeys}
          onSetCapabilitySelection={setCapabilityGrantGroupSelection}
          onToggle={toggleCapabilityGrant}
          footer={(
            <div className="flex flex-wrap gap-2">
              <Button
                disabled={savingCapabilities}
                aria-busy={savingCapabilities}
                onClick={() => void handleSaveCapabilityGrants()}
              >
                {savingCapabilities ? pageT("actions.saving", "Saving...") : pageT("actions.saveCapabilities", "Grant")}
              </Button>
            </div>
          )}
        />
      </div>
    </>
  );
}
