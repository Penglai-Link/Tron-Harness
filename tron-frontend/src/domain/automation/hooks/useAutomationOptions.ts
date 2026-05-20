import {
  useEffect,
  useState,
} from "react";

import { fetchAutomationOptions } from "@/domain/automation/api";
import { getAutomationOptions } from "@/domain/automation/mock/data";
import type { AutomationOptions } from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function useAutomationOptions(): AutomationOptions {
  const dataSource = getServiceDataSource("automation");
  const [options, setOptions] = useState(() => getAutomationOptions());

  useEffect(() => {
    if (dataSource !== "api") {
      setOptions(getAutomationOptions());
      return;
    }

    const controller = new AbortController();
    fetchAutomationOptions(controller.signal)
      .then(setOptions)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[automation] Falling back to mock automation options.", error);
        setOptions(getAutomationOptions());
      });

    return () => controller.abort();
  }, [dataSource]);

  return options;
}
