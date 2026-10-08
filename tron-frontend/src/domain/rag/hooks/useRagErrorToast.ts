import { useEffect, useRef } from "react";
import { useToast } from "@/components/ui/toast";

// Polling failures stay visible in query state; notify once until recovery.
export function useRagErrorToast(message: string | null | undefined) {
  const toast = useToast();
  const previous = useRef<string | null | undefined>(undefined);
  useEffect(() => {
    if (message && message !== previous.current) toast.error(message);
    previous.current = message;
  }, [message, toast]);
}
