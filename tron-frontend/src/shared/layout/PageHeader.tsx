import { Fragment, type ReactNode } from "react";
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
    <header className={cn("mb-6 flex flex-col gap-4", className)}>
      {breadcrumbs?.length ? (
        <nav className="flex flex-wrap items-center text-[12px] text-[var(--color-on-surface-variant)]" aria-label="Breadcrumb">
          {breadcrumbs.map((item, index) => (
            <Fragment key={`${item.label}-${index}`}>
              {item.href ? (
                <Link className="opacity-55" to={item.href}>
                  {item.label}
                </Link>
              ) : (
                <span>{item.label}</span>
              )}
              {index < breadcrumbs.length - 1 ? <span className="mx-2">/</span> : null}
            </Fragment>
          ))}
        </nav>
      ) : null}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p className="min-w-0 text-[16px] font-semibold leading-[1.3] tracking-normal text-[var(--color-on-surface)]">{title}</p>
        {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
      </div>
    </header>
  );
}
