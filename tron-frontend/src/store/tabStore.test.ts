import { afterEach, describe, expect, it } from "vitest";

import { useTabStore } from "./tabStore";

describe("tab creation suppression", () => {
  afterEach(() => {
    useTabStore.getState().clearTabCreationSuppression();
  });

  it("keeps the empty workspace route suppressed until navigation resumes", () => {
    const store = useTabStore.getState();

    store.suppressNextTab();
    expect(useTabStore.getState()).toMatchObject({
      suppressNextTabCreation: true,
      tabCreationSuppressionReachedTarget: false,
      tabCreationSuppressionTargetPath: "/apps",
    });

    useTabStore.getState().markTabCreationSuppressionTargetReached();
    expect(useTabStore.getState().tabCreationSuppressionReachedTarget).toBe(true);

    useTabStore.getState().clearTabCreationSuppression();
    expect(useTabStore.getState()).toMatchObject({
      suppressNextTabCreation: false,
      tabCreationSuppressionReachedTarget: false,
      tabCreationSuppressionTargetPath: null,
    });
  });
});
