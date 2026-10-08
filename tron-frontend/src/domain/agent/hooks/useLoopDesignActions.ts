import { useTranslation } from "react-i18next";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  createLoopDesign,
  updateLoopDesign,
  deleteLoopDesign,
} from "../api/loopApi";
import {
  saveLoopDesignMock,
  deleteLoopDesignMock,
  LoopDesignMockError,
} from "../mock/loopData";
import { LOOP_DESIGNS_CHANGED, type LoopDesign } from "../model/LoopModel";
export function useLoopDesignActions() {
  const source = getServiceDataSource("agent");
  const { t } = useTranslation("pages");
  async function change<T>(operation: () => T | Promise<T>): Promise<T> {
    try {
      const result = await operation();
      window.dispatchEvent(new Event(LOOP_DESIGNS_CHANGED));
      return result;
    } catch (error) {
      if (error instanceof LoopDesignMockError)
        throw new Error(t(`agentLoops.errors.${error.reason}`));
      throw error;
    }
  }
  return {
    save(design: LoopDesign) {
      return change(() =>
        source === "api"
          ? design.version
            ? updateLoopDesign(design)
            : createLoopDesign(design)
          : saveLoopDesignMock(design),
      );
    },
    remove(code: string) {
      return change(() =>
        source === "api" ? deleteLoopDesign(code) : deleteLoopDesignMock(code),
      );
    },
  };
}
