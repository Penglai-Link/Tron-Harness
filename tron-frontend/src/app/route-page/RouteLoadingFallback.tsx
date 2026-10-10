import { useTranslation } from "react-i18next";
import { MotionReveal, ShinyText } from "@/shared/motion";

export default function RouteLoadingFallback({
  fullScreen = false,
}: {
  fullScreen?: boolean;
}) {
  const { t } = useTranslation("common");
  return (
    <section
      aria-busy="true"
      className={
        fullScreen
          ? "flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6"
          : "flex min-h-[calc(100vh-7rem)] items-center justify-center"
      }
    >
      <MotionReveal preset="fade" role="status" className="text-sm text-[var(--color-on-surface-variant)]">
        <ShinyText>{t("table.loading")}</ShinyText>
      </MotionReveal>
    </section>
  );
}
