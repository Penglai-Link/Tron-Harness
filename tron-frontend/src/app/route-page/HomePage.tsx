import { Button } from "@/components/ui/button";
import { Command, ArrowRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";

export default function HomePage() {
  const { t } = useTranslation("home");

  return (
    <main className="min-h-screen bg-[var(--color-background)] px-6 py-8 text-[var(--color-on-surface)]">
      <section className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-6xl flex-col justify-center gap-8">
        <div className="max-w-3xl">
          <div className="mb-4 inline-flex items-center gap-[12px] rounded-full border border-[var(--color-outline-variant)] bg-white px-3 py-1 text-sm text-[var(--color-on-surface-variant)] shadow-sm">
            <Command size={16} aria-hidden="true" />
            {t("badge")}
          </div>
          <h1 className="text-4xl font-semibold leading-tight tracking-normal text-[var(--color-on-surface)]">
            {t("title")}
          </h1>
          <p className="mt-4 max-w-2xl text-base leading-7 text-[var(--color-on-surface-variant)]">
            {t("description")}
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Button render={<Link to="/design-standard" />}>
              {t("cta.designStandard")}
              <ArrowRight size={16} aria-hidden="true" />
            </Button>
            <Button render={<Link to="/" />} variant="secondary">
              {t("cta.sharedShell")}
              <ArrowRight size={16} aria-hidden="true" />
            </Button>
          </div>
        </div>
      </section>
    </main>
  );
}
