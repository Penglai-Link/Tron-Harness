import {
  useCallback,
  useEffect,
  useState,
} from "react";

import { fetchAutomationDetail } from "@/domain/automation/api";
import { getAutomationDetail } from "@/domain/automation/mock/data";
import type { AutomationDetailData } from "@/domain/automation/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export interface AutomationDetailResult {
  automation: AutomationDetailData;
  loading: boolean;
  refresh: () => void;
  setAutomation: (automation: AutomationDetailData) => void;
}

export function useAutomationDetail(automationCode = ""): AutomationDetailResult {
  const dataSource = getServiceDataSource("automation");
  const [reloadKey, setReloadKey] = useState(0);
  const [loading, setLoading] = useState(false);
  const [automation, setAutomation] = useState(() => getAutomationDetail(automationCode));

  useEffect(() => {
    if (dataSource === "api") return;
    setAutomation(getAutomationDetail(automationCode));
  }, [automationCode, dataSource, reloadKey]);

  useEffect(() => {
    if (dataSource !== "api" || !automationCode.trim()) return;

    const controller = new AbortController();
    setLoading(true);

    fetchAutomationDetail(automationCode, controller.signal)
      .then(setAutomation)
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        console.warn("[automation] Falling back to mock automation detail.", error);
        setAutomation(getAutomationDetail(automationCode));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [automationCode, dataSource, reloadKey]);

  const refresh = useCallback(() => setReloadKey((value) => value + 1), []);

  return {
    automation,
    loading,
    refresh,
    setAutomation,
  };
}
