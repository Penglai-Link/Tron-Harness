import { describe, expect, it } from "vitest";

import {
  getNavigationIcon,
  isNavigationIconName,
  mapNavigationTree,
} from "@/shared/lib/navigationMenuMapper";

describe("getNavigationIcon", () => {
  it("resolves PascalCase, kebab-case, and lowercase Lucide names", () => {
    expect(getNavigationIcon(" LayoutDashboard ")).not.toBeNull();
    expect(getNavigationIcon("Bot")).not.toBeNull();
    expect(getNavigationIcon("cat")).not.toBeNull();
    expect(getNavigationIcon("Cat")).not.toBeNull();
    expect(getNavigationIcon("layout-dashboard")).not.toBeNull();
    expect(getNavigationIcon("NotAnIcon")).toBeNull();
  });

  it("validates names against the complete Lucide dynamic icon index", () => {
    expect(isNavigationIconName("cat")).toBe(true);
    expect(isNavigationIconName("NotAnIcon")).toBe(false);
  });
});

describe("mapNavigationTree", () => {
  it("keeps an empty explore entry and mixed widget/page order with their expansion settings", () => {
    const base = { icon: "Ellipsis", label: "探索更多", menuCode: "EXPLORE_MORE", menuType: "Group" as const, pageCode: null, url: null, childDisplay: "Flyout" as const, children: [] };
    const tree = { domainCode: "ADVANCED", locale: "zh-CN", maxDepth: 3, menus: [base] };
    expect(mapNavigationTree(tree)[0]).toMatchObject({ id: "EXPLORE_MORE", flyout: true, items: [] });
    const [section] = mapNavigationTree({ ...tree, menus: [{ ...base, children: [
      { ...base, menuCode: "PROJECTS", menuType: "Widget", pageCode: "chat.projects" },
      { ...base, menuCode: "TASKS", menuType: "Internal", pageCode: "tasks.index", childDisplay: "Inline" },
    ] }] });
    expect(section.items.map((item) => item.menuCode)).toEqual(["PROJECTS", "TASKS"]);
    expect(section.items[0]).toMatchObject({ widgetCode: "chat.projects", childDisplay: "Flyout" });
    expect(section.items[1]).toMatchObject({ href: "/tasks", childDisplay: "Inline" });
  });

  it("excludes observability from persisted workspace parent menus", () => {
    const [section] = mapNavigationTree({ domainCode:"ADVANCED",locale:"zh-CN",maxDepth:3,
      menus:[{menuCode:"OBSERVABILITY",label:"可观测",menuType:"Group",pageCode:null,url:null,icon:"Activity",childDisplay:"Flyout",children:
        ["dashboard","traces","metrics"].map(page=>({menuCode:page,label:page,menuType:"Internal" as const,pageCode:`observability.${page}`,url:null,icon:"Activity",childDisplay:"Inline" as const,children:[]}))
      }] });
    expect(section).toBeUndefined();
  });

  it("excludes standalone observability pages from workspace navigation", () => {
    const [section] = mapNavigationTree({
      domainCode: "ADVANCED",
      locale: "zh-CN",
      maxDepth: 3,
      menus: [{
        children: [],
        icon: "Activity",
        label: "链路",
        menuCode: "TRACES",
        menuType: "Internal",
        pageCode: "observability.traces",
        url: null,
        childDisplay: "Inline",
      }],
    });

    expect(section).toBeUndefined();
  });

  it("maps backend groups, internal pages, and external links to the shell model", () => {
    const [section] = mapNavigationTree({
      domainCode: "ADVANCED",
      locale: "en-US",
      maxDepth: 3,
      menus: [{
        children: [
          {
            children: [],
            icon: "ListTodo",
            label: "Tasks",
            menuCode: "TASKS",
            menuType: "Internal",
            pageCode: "tasks.index",
            url: null,
            childDisplay: "Inline",
          },
          {
            children: [],
            icon: "Globe",
            label: "Documentation",
            menuCode: "DOCS",
            menuType: "ExternalLink",
            pageCode: null,
            url: "https://example.com/docs",
            childDisplay: "Inline",
          },
        ],
        icon: "LayoutDashboard",
        label: "Operations",
        menuCode: "OPERATIONS",
        menuType: "Group",
        pageCode: null,
        url: null,
        childDisplay: "Flyout",
      }],
    });

    expect(section.id).toBe("OPERATIONS");
    expect(section.label).toBe("Operations");
    expect(section.flyout).toBe(true);
    expect(section.items[0].href).toBe("/tasks");
    expect(section.items[1].external).toBe(true);
    expect(section.items[1].href).toBe("https://example.com/docs");
  });

  it.each([
    ["zh-CN", "好兄弟"],
    ["en-US", "Dude"],
  ])("shows the Dude navigation label in %s without the legacy bilingual suffix", (locale, expectedLabel) => {
    const [section] = mapNavigationTree({
      domainCode: "ADVANCED",
      locale,
      maxDepth: 1,
      menus: [{
        children: [],
        icon: "Bot",
        label: "Dude（好兄弟）",
        menuCode: "DUDES",
        menuType: "Internal",
        pageCode: "dudes.index",
        url: null,
        childDisplay: "Inline",
      }],
    });

    expect(section.items[0].label).toBe(expectedLabel);
  });
});
