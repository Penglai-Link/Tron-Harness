import type { ReactNode } from "react";
import { Link } from "react-router";

import { cn } from "@/shared/lib/utils";

export interface BreadcrumbItem {
  label: string;
  href?: string;
}

interface PageHeaderProps {
  actions?: ReactNode;
  breadcrumbs?: BreadcrumbItem[];
  className?: string;
  title: string;
}

export function PageHeader({ actions, breadcrumbs, className, title }: PageHeaderProps) {
  return (
    <header className={cn("mb-6 flex flex-wrap items-center justify-between gap-4", className)}>
      <div className="min-w-0">
        {breadcrumbs?.length ? (
          <nav className="mb-2 flex flex-wrap items-center gap-1 text-[12px] text-[var(--color-on-surface-variant)]" aria-label="Breadcrumb">
            {breadcrumbs.map((item, index) => (
              <span className="flex items-center gap-1" key={`${item.label}-${index}`}>
                {item.href ? (
                  <Link className="hover:text-[var(--color-on-surface)]" to={item.href}>
                    {item.label}
                  </Link>
                ) : (
                  <span>{item.label}</span>
                )}
                {index < breadcrumbs.length - 1 ? <span>/</span> : null}
              </span>
            ))}
          </nav>
        ) : null}
        <h1 className={cn("text-2xl font-semibold leading-[1.3] tracking-normal text-[var(--color-on-surface)]", breadcrumbs?.length ? "mt-1" : undefined)}>{title}</h1>
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
    </header>
  );
}
