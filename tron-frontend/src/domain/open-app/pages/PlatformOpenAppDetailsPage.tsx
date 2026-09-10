import {
  useEffect,
  useState,
} from "react";
import {
  Navigate,
  useLocation,
  useNavigate,
  useSearchParams,
} from "react-router";
import { Edit3 } from "lucide-react";

import {
  fetchPlatformOpenApp,
  fetchPlatformOpenAppCatalog,
  rotatePlatformOpenAppKey,
} from "@/domain/open-app/api";
import {
  OpenAppCapabilityGrantSummarySection,
  OpenAppMetadataSummarySection,
  OpenAppSecretRevealCard,
  OpenAppUserGrantSummarySection,
  useOpenAppPageT,
} from "@/domain/open-app/components";
import {
  getPlatformOpenAppMock,
  listPlatformOpenAppCatalogMock,
  rotatePlatformOpenAppKeyMock,
} from "@/domain/open-app/mock";
import type {
  ManagedOpenAppDetail,
  ManagedOpenAppSecretReveal,
  OpenApiCapabilityCatalogGroup,
} from "@/domain/open-app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";

interface OpenAppDetailLocationState {
  secretReveal?: ManagedOpenAppSecretReveal;
}

export function PlatformOpenAppDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const location = useLocation();
  const toast = useToast();
  const pageT = useOpenAppPageT();
  const dataSource = getServiceDataSource("admin");
  const openAppCode = searchParams.get("openAppCode") ?? "";
  const [detail, setDetail] = useState<ManagedOpenAppDetail | null>(null);
  const [catalog, setCatalog] = useState<OpenApiCapabilityCatalogGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [secretReveal, setSecretReveal] = useState<ManagedOpenAppSecretReveal | null>(null);
  const [rotateDialogOpen, setRotateDialogOpen] = useState(false);
  const [rotatePending, setRotatePending] = useState(false);
  const [copyPending, setCopyPending] = useState(false);

  useEffect(() => {
    const state = location.state as OpenAppDetailLocationState | null;
    if (!state?.secretReveal) return;

    setSecretReveal(state.secretReveal);
    navigate(`${location.pathname}${location.search}`, { replace: true });
  }, [location.pathname, location.search, location.state, navigate]);

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

  async function handleRotateKey() {
    if (!detail) return;
    setRotatePending(true);
    try {
      const result = dataSource === "api"
        ? await rotatePlatformOpenAppKey(detail.openAppCode)
        : rotatePlatformOpenAppKeyMock(detail.openAppCode);
      setDetail(result.openApp);
      setSecretReveal(result.secretReveal);
      setRotateDialogOpen(false);
      toast.success(pageT("messages.rotated", "App key rotated"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.rotate", "App key could not be rotated"));
    } finally {
      setRotatePending(false);
    }
  }

  async function handleCopySecret() {
    if (!secretReveal) return;
    setCopyPending(true);
    try {
      await navigator.clipboard.writeText(secretReveal.appKey);
      toast.success(pageT("messages.copied", "App key copied"));
    } catch {
      toast.error(pageT("errors.copy", "App key could not be copied"));
    } finally {
      setCopyPending(false);
    }
  }

  if (loading && !detail) {
    return (
      <>
        <PageHeader
          breadcrumbs={[
            { label: pageT("breadcrumbs.list", "OpenAPI"), href: "/settings/openapi" },
            { label: pageT("breadcrumbs.loading", "Loading") },
          ]}
          title={pageT("title.detail", "Open App Detail")}
        />
        <SurfaceCard>{pageT("detail.loading", "Loading open-app details...")}</SurfaceCard>
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
          title={pageT("title.detail", "Open App Detail")}
        />
        <SurfaceCard>{pageT("detail.notFound", "Open-app not found.")}</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: pageT("breadcrumbs.list", "OpenAPI"), href: "/settings/openapi" },
          { label: detail.name },
        ]}
        title={detail.name}
        actions={(
          <>
            <Button
              className="min-w-28"
              variant="secondary"
              onClick={() => navigate(`/settings/openapi/edit?openAppCode=${encodeURIComponent(detail.openAppCode)}`)}
            >
              <Edit3 size={16} aria-hidden="true" />
              {pageT("actions.edit", "Edit")}
            </Button>
            <Button className="min-w-28" onClick={() => setRotateDialogOpen(true)}>
              {pageT("actions.rotateKey", "Rotate key")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-6">
        {secretReveal ? (
          <OpenAppSecretRevealCard
            copyPending={copyPending}
            secretReveal={secretReveal}
            onCopy={() => void handleCopySecret()}
            onDismiss={() => setSecretReveal(null)}
          />
        ) : null}

        <OpenAppMetadataSummarySection detail={detail} />
        <OpenAppUserGrantSummarySection detail={detail} />
        <OpenAppCapabilityGrantSummarySection catalog={catalog} detail={detail} />
      </div>

      <ConfirmDialog
        open={rotateDialogOpen}
        pending={rotatePending}
        title={pageT("rotateDialog.title", "Rotate app key?")}
        description={pageT("rotateDialog.description", "The current app key becomes invalid immediately. The new key will be revealed once after rotation.")}
        confirmText={rotatePending ? pageT("rotateDialog.confirming", "Rotating...") : pageT("actions.rotateKey", "Rotate key")}
        cancelText={pageT("rotateDialog.cancel", "Keep current key")}
        onCancel={() => setRotateDialogOpen(false)}
        onConfirm={() => void handleRotateKey()}
      />
    </>
  );
}
