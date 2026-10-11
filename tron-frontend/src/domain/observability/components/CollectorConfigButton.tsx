import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { getCollectorExportConfig } from "../api/observabilityApi";

export function CollectorConfigButton() {
  const { t } = useTranslation("observability");
  const toast = useToast();
  const copy = async () => {
    try {
      const config = await getCollectorExportConfig();
      await navigator.clipboard.writeText(JSON.stringify({
        OTEL_EXPORTER_OTLP_ENDPOINT: config.endpoint,
        OTEL_EXPORTER_OTLP_PROTOCOL: "http/protobuf",
        OTEL_EXPORTER_OTLP_HEADERS: `authorization=${encodeURIComponent(config.authorization)}`,
      }, null, 2));
      toast.success(t("copied"));
    } catch {
      toast.error(t("copyFailed"));
    }
  };
  return <Button variant="secondary" onClick={() => void copy()}>{t("copyCollectorConfig")}</Button>;
}
