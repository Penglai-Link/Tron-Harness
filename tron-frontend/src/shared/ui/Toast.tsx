import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  CheckCircle2,
  Info,
  TriangleAlert,
  X,
  XCircle,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";

export type ToastTone = "info" | "success" | "warning" | "error";

export interface ToastMessage {
  id: string;
  tone: ToastTone;
  title?: string;
  message: string;
  durationMs?: number;
}

export interface ToastOptions {
  title?: string;
  durationMs?: number;
}

export interface ToastShowOptions extends ToastOptions {
  tone?: ToastTone;
  message: string;
}

export interface ToastApi {
  show: (toast: ToastShowOptions) => string;
  success: (message: string, options?: ToastOptions) => string;
  error: (message: string, options?: ToastOptions) => string;
  warning: (message: string, options?: ToastOptions) => string;
  info: (message: string, options?: ToastOptions) => string;
  dismiss: (id: string) => void;
}

export interface ToastProviderProps {
  children: ReactNode;
  defaultDurationMs?: number;
}

const ToastContext = createContext<ToastApi | undefined>(undefined);
const DEFAULT_TOAST_DURATION_MS = 2400;

const toneClasses: Record<ToastTone, string> = {
  info: "border-[var(--color-accent-container)] bg-white text-[var(--color-on-surface)]",
  success: "border-[var(--color-success-container)] bg-[var(--color-success-container)] text-[var(--color-on-success-container)]",
  warning: "border-[var(--color-warning-container)] bg-[var(--color-warning-container)] text-[var(--color-on-warning-container)]",
  error: "border-[var(--color-error-container)] bg-[var(--color-error-container)] text-[var(--color-on-error-container)]",
};

const toneIcons: Record<ToastTone, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: TriangleAlert,
  error: XCircle,
};

function createToastId() {
  return `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function ToastProvider({ children, defaultDurationMs = DEFAULT_TOAST_DURATION_MS }: ToastProviderProps) {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());

  const dismiss = useCallback((id: string) => {
    const timer = timers.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timers.current.delete(id);
    }
    setToasts((currentToasts) => currentToasts.filter((toast) => toast.id !== id));
  }, []);

  const show = useCallback((toast: ToastShowOptions) => {
    const id = createToastId();
    const durationMs = toast.durationMs ?? defaultDurationMs;
    const nextToast: ToastMessage = {
      id,
      tone: toast.tone ?? "info",
      title: toast.title,
      message: toast.message,
      durationMs,
    };

    setToasts((currentToasts) => [...currentToasts, nextToast].slice(-4));

    if (durationMs > 0) {
      const timer = setTimeout(() => {
        timers.current.delete(id);
        setToasts((currentToasts) => currentToasts.filter((item) => item.id !== id));
      }, durationMs);
      timers.current.set(id, timer);
    }

    return id;
  }, [defaultDurationMs]);

  const api = useMemo<ToastApi>(() => ({
    show,
    success: (message, options) => show({ ...options, message, tone: "success" }),
    error: (message, options) => show({ ...options, message, tone: "error" }),
    warning: (message, options) => show({ ...options, message, tone: "warning" }),
    info: (message, options) => show({ ...options, message, tone: "info" }),
    dismiss,
  }), [dismiss, show]);

  useEffect(() => () => {
    timers.current.forEach((timer) => clearTimeout(timer));
    timers.current.clear();
  }, []);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="fixed right-4 top-4 z-[80] flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-3" aria-label="Notifications">
        {toasts.map((toast) => {
          const Icon = toneIcons[toast.tone];

          return (
            <div
              className={cn(
                "grid grid-cols-[auto_minmax(0,1fr)_auto] gap-3 rounded-lg border px-4 py-3 text-sm shadow-[0_18px_40px_rgba(0,0,0,0.14)]",
                toneClasses[toast.tone],
              )}
              key={toast.id}
              role="status"
            >
              <Icon className="mt-0.5 shrink-0" size={18} aria-hidden="true" />
              <div className="min-w-0">
                {toast.title ? <p className="font-semibold leading-5">{toast.title}</p> : null}
                <p className="leading-5">{toast.message}</p>
              </div>
              <button
                aria-label="Dismiss toast"
                className="inline-flex size-8 shrink-0 items-center justify-center rounded-lg text-current opacity-70 transition hover:bg-black/5 hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
                type="button"
                onClick={() => dismiss(toast.id)}
              >
                <X size={16} aria-hidden="true" />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);

  if (!context) {
    throw new Error("useToast must be used within ToastProvider.");
  }

  return context;
}
