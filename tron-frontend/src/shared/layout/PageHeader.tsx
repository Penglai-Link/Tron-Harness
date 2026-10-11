import { Fragment, type ReactNode } from "react";
import { Link } from "react-router";

import { cn } from "@/shared/lib/utils";

export interface BreadcrumbItem {
  label: string;
  href?: string;
  state?: unknown;
}

interface PageHeaderProps {
  actions?: ReactNode;
  breadcrumbs?: BreadcrumbItem[];
  className?: string;
  title: string;
}

export function PageHeader({ actions, breadcrumbs, className }: PageHeaderProps) {
  const visibleBreadcrumbs = breadcrumbs ?? [];
  const hasBreadcrumbs = visibleBreadcrumbs.length > 0;
  const hasActions = Boolean(actions);

  if (!hasBreadcrumbs && !hasActions) {
    return null;
  }

  return (
    <header className={cn("mb-6 flex flex-col gap-4", className)}>
      {hasBreadcrumbs ? (
        <nav className="flex flex-wrap items-center text-[12px] text-[var(--color-on-surface-variant)]" aria-label="Breadcrumb">
          {visibleBreadcrumbs.map((item, index) => (
            <Fragment key={`${item.label}-${index}`}>
              {item.href ? (
                <Link className="opacity-55" to={item.href} state={item.state}>
                  {item.label}
                </Link>
              ) : (
                <span>{item.label}</span>
              )}
              {index < visibleBreadcrumbs.length - 1 ? <span className="mx-2">/</span> : null}
            </Fragment>
          ))}
        </nav>
      ) : null}
      {hasActions ? <div className="flex flex-wrap items-center justify-end gap-[12px]">{actions}</div> : null}
    </header>
  );
}
