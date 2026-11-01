import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/shared";

export function ProviderPageHeader({ title, actions }: { title: string; actions: ReactNode }) {
  const { t } = useTranslation("pages");
  return <PageHeader title={title} breadcrumbs={[{ label: t("modelProviders.titles.providers"), href: "/settings/model-providers" }, { label: title }]} actions={actions} />;
}
