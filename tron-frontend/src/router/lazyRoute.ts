import { lazy, type ComponentType } from "react";

export const ROUTE_LOAD_TIMEOUT_MS = 20_000;

/** A pending module request must eventually reach the route error boundary. */
export function lazyRoute<T extends ComponentType>(load: () => Promise<{ default: T }>) {
  return lazy(() => new Promise<{ default: T }>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error("Route module loading timed out"));
    }, ROUTE_LOAD_TIMEOUT_MS);
    // Also turn a synchronous loader failure into the normal rejection path.
    Promise.resolve().then(load).then(
      (module) => {
        clearTimeout(timer);
        resolve(module);
      },
      (error: unknown) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  }));
}
