import {
  createContext,
  forwardRef,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type AnchorHTMLAttributes,
  type MouseEvent,
  type ReactNode,
} from "react";

export interface ClientRouteLocation {
  hash: string;
  pathname: string;
  search: string;
}

interface ClientRouterContextValue {
  location: ClientRouteLocation;
  navigate: (href: string, options?: { replace?: boolean }) => void;
}

const ClientRouterContext = createContext<ClientRouterContextValue | null>(null);

function readLocation(): ClientRouteLocation {
  return {
    hash: window.location.hash,
    pathname: window.location.pathname,
    search: window.location.search,
  };
}

function isModifiedClick(event: MouseEvent<HTMLAnchorElement>) {
  return event.metaKey || event.altKey || event.ctrlKey || event.shiftKey || event.button !== 0;
}

function isInternalHref(href: string) {
  return href.startsWith("/") && !href.startsWith("//");
}

export function ClientRouterProvider({ children }: { children: ReactNode }) {
  const [location, setLocation] = useState<ClientRouteLocation>(() => readLocation());

  useEffect(() => {
    function handlePopState() {
      setLocation(readLocation());
    }

    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  const navigate = useCallback((href: string, options?: { replace?: boolean }) => {
    if (!isInternalHref(href)) {
      window.location.assign(href);
      return;
    }

    const nextUrl = new URL(href, window.location.href);
    const nextPath = `${nextUrl.pathname}${nextUrl.search}${nextUrl.hash}`;
    const currentPath = `${window.location.pathname}${window.location.search}${window.location.hash}`;

    if (nextPath !== currentPath) {
      if (options?.replace) {
        window.history.replaceState(null, "", nextPath);
      } else {
        window.history.pushState(null, "", nextPath);
      }
    }

    setLocation(readLocation());
  }, []);

  const value = useMemo(() => ({ location, navigate }), [location, navigate]);

  return <ClientRouterContext.Provider value={value}>{children}</ClientRouterContext.Provider>;
}

export function useRouteLocation() {
  const context = useContext(ClientRouterContext);
  return context?.location ?? readLocation();
}

export function useNavigate() {
  const context = useContext(ClientRouterContext);

  return useCallback(
    (href: string, options?: { replace?: boolean }) => {
      if (context) {
        context.navigate(href, options);
      } else if (isInternalHref(href)) {
        window.history.pushState(null, "", href);
        window.dispatchEvent(new PopStateEvent("popstate"));
      } else {
        window.location.assign(href);
      }
    },
    [context],
  );
}

export const ClientLink = forwardRef<HTMLAnchorElement, AnchorHTMLAttributes<HTMLAnchorElement> & { href: string }>(
  ({ href, onClick, target, ...props }, ref) => {
    const navigate = useNavigate();

    return (
      <a
        {...props}
        href={href}
        onClick={(event) => {
          onClick?.(event);

          if (event.defaultPrevented || target === "_blank" || isModifiedClick(event) || !isInternalHref(href)) {
            return;
          }

          event.preventDefault();
          navigate(href);
        }}
        ref={ref}
        target={target}
      />
    );
  },
);

ClientLink.displayName = "ClientLink";
