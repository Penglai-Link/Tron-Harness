import { useTranslation } from "react-i18next";

export default function ConsoleFallbackPage() {
  const { t } = useTranslation("console");

  return (
    <section className="flex min-h-[calc(100vh-7rem)] items-center justify-center">
      <div className="max-w-xl text-center">
        <h1 className="text-2xl font-semibold">{t("fallback.title")}</h1>
        <p className="mt-3 text-sm leading-6 text-[var(--color-on-surface-variant)]">
          {t("fallback.description")}
        </p>
      </div>
    </section>
  );
}
