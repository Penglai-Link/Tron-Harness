import { afterEach, expect, it } from "vitest";
import { useTabStore } from "./tabStore";
import { useFileTabsStore } from "./fileTabsStore";

afterEach(() => useTabStore.getState().resetAllTabs());

it("retains independent file selection per session and disposes only closed owners", () => {
  const tabs = useTabStore.getState();
  const a = tabs.addTab({ path: "/chats?sessionCode=a", title: "A", closable: true });
  const b = tabs.addTab({ path: "/chats?sessionCode=b", title: "B", closable: true });
  const files = useFileTabsStore.getState();
  const a1 = files.openFileTab(a);
  const a2 = files.openFileTab(a);
  const b1 = files.openFileTab(b);
  files.setActiveFileTab(a1, a);
  tabs.setActiveTab(a);
  expect(useFileTabsStore.getState().activeKeysByOwner).toMatchObject({ [a]: a1, [b]: b1 });
  files.setActiveFileTab(null, b);
  tabs.setActiveTab(b);
  tabs.setActiveTab(a);
  expect(useFileTabsStore.getState().activeKeysByOwner[a]).toBe(a1);
  files.closeFileTab(a1);
  expect(useFileTabsStore.getState().activeKeysByOwner[a]).toBe(a2);
  expect(useFileTabsStore.getState().activeKeysByOwner[b]).toBeNull();
  tabs.closeTab(a);
  expect(useFileTabsStore.getState().fileTabKeys).toEqual([b1]);
});
