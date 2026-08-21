import {
  Bot,
  ChevronDown,
  Command,
  Gauge,
  Search,
  ShieldCheck,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  componentPrinciples,
  tronColors,
  typeScale,
} from "@/shared/constants/designTokens";
import {
  designStandardSidebarItems,
  designStandardTableRows,
  designStandardSignalCards,
} from "@/domain/design-standard/constants";


export function DesignStandardPage() {
  const { t } = useTranslation(["pages"]);
  return (
    <main className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <div className="grid min-h-screen grid-cols-1 lg:grid-cols-[240px_1fr]">
        <aside className="border-r border-[var(--color-outline-variant)] bg-white px-4 py-5">
          <div className="flex items-center gap-3 px-2">
            <div className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-primary)] text-[var(--color-on-primary)]">
              <Command size={18} aria-hidden="true" />
            </div>
            <div>
              <p className="text-sm font-semibold">Tron</p>
              <p className="text-xs text-[var(--color-on-surface-variant)]">{t("pages:designStandard.label", "Design Standard")}</p>
            </div>
          </div>
          <nav className="mt-8 space-y-1" aria-label="Design standard preview navigation">
            {designStandardSidebarItems.map((item) => (
              <a
                className="flex h-9 items-center justify-between rounded-lg px-3 text-sm text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] data-[active=true]:bg-[var(--color-surface-container-high)] data-[active=true]:text-[var(--color-on-surface)]"
                data-active={item === "Apps"}
                href="#canvas"
                key={item}
              >
                {item}
                {item === "Apps" ? <ChevronDown size={15} aria-hidden="true" /> : null}
              </a>
            ))}
          </nav>
        </aside>

        <section className="min-w-0">
          <header className="sticky top-0 z-20 border-b border-[var(--color-outline-variant)] bg-[rgba(253,248,248,0.88)] px-6 py-4 backdrop-blur">
            <div className="flex flex-wrap items-center justify-between gap-4">
              <div>
                <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                  {t("pages:designStandard.eyebrow", "Design system / Tron light")}
                </p>
                <h1 className="mt-1 text-2xl font-semibold tracking-normal">{t("pages:designStandard.title", "Workspace UI foundation")}</h1>
              </div>
              <div className="flex items-center gap-2">
                <Button size="icon" variant="secondary" aria-label={t("pages:designStandard.search", "Search design standard")}>
                  <Search size={17} aria-hidden="true" />
                </Button>
                <Button>
                  <ShieldCheck size={16} aria-hidden="true" />
                  {t("pages:designStandard.ready", "Ready for onboarding")}
                </Button>
              </div>
            </div>
          </header>

          <div id="canvas" className="mx-auto max-w-7xl space-y-8 px-6 py-8">
            <section className="grid gap-4 xl:grid-cols-[1fr_360px]">
              <div className="rounded-2xl border border-[var(--color-card-border)] bg-white p-6 shadow-[0_18px_50px_rgba(0,0,0,0.05)]">
                <div className="flex flex-wrap items-start justify-between gap-4">
                  <div>
                    <Badge tone="info">{t("pages:designStandard.badge", "Apple-like enterprise SaaS")}</Badge>
                    <h2 className="mt-4 text-[40px] font-semibold leading-[1.1] tracking-normal">{t("pages:designStandard.heroTitle", "Calm control for agent operations")}</h2>
                    <p className="mt-4 max-w-2xl text-base leading-7 text-[var(--color-on-surface-variant)]">
                      {t("pages:designStandard.heroDescription", "The Tron light system uses restrained color, dense but breathable spacing, and stable workspace navigation for AI agent governance.")}
                    </p>
                  </div>
                  <div className="flex size-14 items-center justify-center rounded-2xl bg-[var(--color-accent-container)] text-[var(--color-on-accent-container)]">
                    <Bot size={26} aria-hidden="true" />
                  </div>
                </div>

                <div className="mt-8 grid gap-3 md:grid-cols-3">
                  {designStandardSignalCards.map((item) => (
                    <div className="rounded-xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-lowest)] p-4" key={item}>
                      <p className="text-sm font-semibold">{item}</p>
                      <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">
                        {t("pages:designStandard.signalDescription", "Baseline pattern reserved for shared shell and later Stitch page implementation.")}
                      </p>
                    </div>
                  ))}
                </div>
              </div>

              <div className="rounded-2xl border border-[var(--color-card-border)] bg-white p-5 shadow-[0_18px_50px_rgba(0,0,0,0.04)]">
                <div className="flex items-center justify-between">
                  <h2 className="text-xl font-semibold">{t("pages:designStandard.componentSignals", "Component signals")}</h2>
                  <Gauge size={20} className="text-[var(--color-accent)]" aria-hidden="true" />
                </div>
                <div className="mt-5 space-y-3">
                  {componentPrinciples.map((principle) => (
                    <div className="rounded-xl bg-[var(--color-surface-container-low)] px-4 py-3 text-sm text-[var(--color-on-surface-variant)]" key={principle}>
                      {principle}
                    </div>
                  ))}
                </div>
              </div>
            </section>

            <section className="grid gap-4 xl:grid-cols-[420px_1fr]">
              <div className="rounded-2xl border border-[var(--color-card-border)] bg-white p-5">
                <h2 className="text-xl font-semibold">{t("pages:designStandard.colorRoles", "Color roles")}</h2>
                <div className="mt-5 grid gap-3 sm:grid-cols-2">
                  {tronColors.map((color) => (
                    <div className="rounded-xl border border-[var(--color-card-border)] p-3" key={color.token}>
                      <div className="h-16 rounded-lg border border-black/5" style={{ backgroundColor: color.value }} />
                      <p className="mt-3 text-sm font-semibold">{color.name}</p>
                      <p className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">{color.value}</p>
                      <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">{color.role}</p>
                    </div>
                  ))}
                </div>
              </div>

              <div className="space-y-4">
                <section className="rounded-2xl border border-[var(--color-card-border)] bg-white p-5">
                  <h2 className="text-xl font-semibold">{t("pages:designStandard.typography", "Typography scale")}</h2>
                  <div className="mt-4 divide-y divide-[var(--color-outline-variant)]">
                    {typeScale.map((type) => (
                      <div className="grid gap-3 py-4 lg:grid-cols-[190px_1fr_180px]" key={type.name}>
                        <p className="text-sm font-semibold">{type.name}</p>
                        <p className={type.className}>Inter / {type.name}</p>
                        <p className="text-[13px] text-[var(--color-on-surface-variant)]">{type.usage}</p>
                      </div>
                    ))}
                  </div>
                </section>

                <section className="overflow-hidden rounded-2xl border border-[var(--color-card-border)] bg-white">
                  <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
                    <div>
                      <h2 className="text-xl font-semibold">{t("pages:designStandard.tableTitle", "Dense table pattern")}</h2>
                      <p className="text-[13px] text-[var(--color-on-surface-variant)]">{t("pages:designStandard.tableDescription", "Token-governed rows for workspace resource views.")}</p>
                    </div>
                    <Button variant="secondary" size="sm">{t("pages:designStandard.export", "Export")}</Button>
                  </div>
                  <div className="overflow-x-auto">
                    <table className="w-full min-w-[640px] border-collapse text-left text-sm">
                      <thead>
                        <tr className="border-b border-[var(--color-outline-variant)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                          <th className="px-5 py-3">{t("pages:designStandard.columns.workspace", "Workspace")}</th>
                          <th className="px-5 py-3">{t("pages:designStandard.columns.source", "Source")}</th>
                          <th className="px-5 py-3">{t("pages:designStandard.columns.model", "Model")}</th>
                          <th className="px-5 py-3">{t("pages:designStandard.columns.usage", "Usage")}</th>
                          <th className="px-5 py-3">{t("pages:designStandard.columns.status", "Status")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {designStandardTableRows.map((row) => (
                          <tr className="border-b border-[var(--color-outline-variant)] last:border-0" key={row.workspace}>
                            <td className="px-5 py-3 font-medium">{row.workspace}</td>
                            <td className="px-5 py-3 text-[var(--color-on-surface-variant)]">{row.source}</td>
                            <td className="px-5 py-3 text-[var(--color-on-surface-variant)]">{row.model}</td>
                            <td className="px-5 py-3 font-mono text-[13px]">{row.usage}</td>
                            <td className="px-5 py-3">
                              <Badge tone={row.status === "Healthy" ? "success" : "warning"}>{row.status}</Badge>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </section>
              </div>
            </section>
          </div>
        </section>
      </div>
    </main>
  );
}
