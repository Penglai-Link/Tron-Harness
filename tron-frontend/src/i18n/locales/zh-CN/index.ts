import { common } from "./common";
import { consoleMessages } from "./console";
import { pages } from "./pages";
import { shell } from "./shell";

export const zhCN = {
  common,
  console: consoleMessages,
  pages,
  shell,
} as const;
