import { toastManager } from "@/components/ui/toast";
import { useCallback, useMemo } from "react";

function createToast(type: 'success' | 'error' | 'warning' | 'info') {
  return (description: string, config?: { title?: string | null, durationMs?: number }) => {
    const { title, durationMs } = config || {}
    return toastManager.add({
      type,
      description,
      title,
      timeout: durationMs
    })
  }
}


export function useToast() {
  const dismiss = useCallback((id: string) => toastManager.close(id), [])
  return useMemo(() => ({
    warning: createToast('warning'),
    info: createToast('info'),
    success: createToast('success'),
    error: createToast('error'),
    dismiss
  }), [])
}
