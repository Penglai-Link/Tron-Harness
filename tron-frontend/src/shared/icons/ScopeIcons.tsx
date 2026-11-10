import type { SVGProps } from "react";

type ScopeIconProps = SVGProps<SVGSVGElement>;

/** An open spatial volume for the workspace scope. */
export function WorkspaceIcon(props: ScopeIconProps) {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false" {...props}>
      <path d="m12 3 9 5v8l-9 5-9-5V8l9-5Z" />
      <path d="m3 8 9 5 9-5M12 13v8" />
      <path d="m7.5 5.5 9 5" />
    </svg>
  );
}

/** A compass around a shared center for the domain scope. */
export function DomainIcon(props: ScopeIconProps) {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false" {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 2v3m10 7h-3m-7 10v-3M2 12h3" />
      <path d="m12 8 4 4-4 4-4-4 4-4Z" />
    </svg>
  );
}
