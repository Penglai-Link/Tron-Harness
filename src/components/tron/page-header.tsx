import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export interface BreadcrumbItem {
  label: string;
  href?: string;
}

interface PageHeaderProps {
  actions?: ReactNode;
  breadcrumbs?: BreadcrumbItem[];
  className?: string;
  description?: string;
  eyebrow?: string;
  status?: string;
  title: string;
}

export function PageHeader({ actions, breadcrumbs, className, description, eyebrow, status, title }: PageHeaderProps) {
  return (
    <header className={cn("mb-6 flex flex-wrap items-start justify-between gap-4", className)}>
      <div className="min-w-0">
        {breadcrumbs?.length ? (
          <nav className="mb-2 flex flex-wrap items-center gap-1 text-[12px] text-[var(--color-on-surface-variant)]" aria-label="Breadcrumb">
            {breadcrumbs.map((item, index) => (
              <span className="flex items-center gap-1" key={`${item.label}-${index}`}>
                {item.href ? (
                  <a className="hover:text-[var(--color-on-surface)]" href={item.href}>
                    {item.label}
                  </a>
                ) : (
                  <span>{item.label}</span>
                )}
                {index < breadcrumbs.length - 1 ? <span>/</span> : null}
              </span>
            ))}
          </nav>
        ) : null}
        <div className="flex flex-wrap items-center gap-3">
          {eyebrow ? <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{eyebrow}</p> : null}
          {status ? <Badge tone="accent">{status}</Badge> : null}
        </div>
        <h1 className="mt-1 text-2xl font-semibold leading-[1.3] tracking-normal text-[var(--color-on-surface)]">{title}</h1>
        {description ? <p className="mt-2 max-w-3xl text-sm leading-6 text-[var(--color-on-surface-variant)]">{description}</p> : null}
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
    </header>
  );
}

